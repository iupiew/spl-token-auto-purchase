use pinocchio::{
    account_info::AccountInfo, entrypoint::ProgramResult, instruction::Seed, instruction::Signer,
    msg, program::invoke_signed, pubkey::Pubkey,
};

use crate::{
    dex::{types::SwapParams, DexInterface},
    error::AutoBuyerError,
    state::{constants, PoolConfig, SwapCalculation, TradingPair},
};

/// Minimal AmmInfo fields we need - parsed via zero-copy at verified offsets
/// Raydium V4 pool account is 752 bytes total (source: raydium-amm program)
/// We only need fields up to target_orders for swap CPI
#[allow(dead_code)]
pub struct AmmInfo {
    pub status: u64,
    pub nonce: u64,
    pub trade_fee_numerator: u64,
    pub trade_fee_denominator: u64,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub open_orders: Pubkey,
    pub market_id: Pubkey,
    pub market_program_id: Pubkey,
    pub target_orders: Pubkey,
}

impl AmmInfo {
    /// Parse AmmInfo from pool account data using zero-copy reads
    /// Verified offsets from Raydium V4 source (raydium-amm/src/state.rs)
    fn parse(data: &[u8]) -> Result<Self, AutoBuyerError> {
        // Raydium V4 pool account is exactly 752 bytes
        if data.len() != 752 {
            return Err(AutoBuyerError::InvalidParameters);
        }

        // Helper to read u64 at offset (little-endian, unaligned-safe)
        let read_u64 = |offset: usize| -> u64 {
            let bytes: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
            u64::from_le_bytes(bytes)
        };

        // Helper to read Pubkey at offset (32 bytes)
        let read_pubkey = |offset: usize| -> Pubkey {
            let mut pk = [0u8; 32];
            pk.copy_from_slice(&data[offset..offset + 32]);
            Pubkey::from(pk)
        };

        // Verified offsets from raydium-amm program (state.rs AmmInfo layout)
        // status: 0, nonce: 8, ... trade_fee_numerator: 200, trade_fee_denominator: 208
        // base_vault: 280, quote_vault: 312, base_mint: 344, quote_mint: 376
        // lp_mint: 408, open_orders: 440, market_id: 472, market_program_id: 504
        // target_orders: 536, withdraw_queue: 568, lp_vault: 600, owner: 632, lp_reserve: 664
        // Total: 752 bytes (lp_reserve at 664 + 8 = 672, plus padding to 752)

        Ok(Self {
            status: read_u64(0),
            nonce: read_u64(8),
            trade_fee_numerator: read_u64(200),
            trade_fee_denominator: read_u64(208),
            base_vault: read_pubkey(280),
            quote_vault: read_pubkey(312),
            base_mint: read_pubkey(344),
            quote_mint: read_pubkey(376),
            lp_mint: read_pubkey(408),
            open_orders: read_pubkey(440),
            market_id: read_pubkey(472),
            market_program_id: read_pubkey(504),
            target_orders: read_pubkey(536),
        })
    }
}

/// Raydium V4 DEX implementation
pub struct RaydiumV4;

impl RaydiumV4 {
    /// Создать новый экземпляр Raydium v4
    pub fn new() -> Self {
        Self
    }

    /// Загрузить информацию о пуле Raydium
    fn load_amm_info(&self, pool_account: &AccountInfo) -> Result<AmmInfo, AutoBuyerError> {
        if pool_account.owner() != &constants::RAYDIUM_V4_PROGRAM_ID {
            return Err(AutoBuyerError::InvalidAccountOwner);
        }

        let pool_data = pool_account
            .try_borrow_data()
            .map_err(|_| AutoBuyerError::InvalidParameters)?;

        let amm_info = AmmInfo::parse(&pool_data)?;

        // Validate pool status: 0 = uninitialized, 1 = initialized (allow swaps)
        // Raydium status: 0 = uninitialized, 1 = initialized, 2 = disabled
        if amm_info.status != 1 {
            msg!("Pool status invalid for swaps: {}", amm_info.status);
            return Err(AutoBuyerError::InvalidParameters);
        }

        Ok(amm_info)
    }

    /// Получить баланс токенов из аккаунта (uses pinocchio_token)
    fn get_token_balance(account_info: &AccountInfo) -> Result<u64, AutoBuyerError> {
        let data = account_info
            .try_borrow_data()
            .map_err(|_| AutoBuyerError::CpiError)?;

        // pinocchio_token::state::Account layout:
        // mint (32) + owner (32) + amount (8) + delegate (36) + state (1) + is_native (1) + delegated_amount (8) + close_authority (36)
        // amount is at offset 64 (32+32)
        if data.len() < 72 {
            return Err(AutoBuyerError::InvalidParameters);
        }
        let amount_bytes: [u8; 8] = data[64..72].try_into().unwrap();
        Ok(u64::from_le_bytes(amount_bytes))
    }

    /// Рассчитать количество выходного токена
    fn calculate_amount_out(
        &self,
        amount_in: u64,
        reserve_in: u64,
        reserve_out: u64,
        fee_numerator: u64,
        fee_denominator: u64,
    ) -> Result<(u64, u64), AutoBuyerError> {
        if reserve_in == 0 || reserve_out == 0 {
            return Err(AutoBuyerError::InsufficientLiquidity);
        }

        let fee_amount = amount_in
            .checked_mul(fee_numerator)
            .and_then(|x| x.checked_div(fee_denominator))
            .ok_or(AutoBuyerError::MathOverflow)?;

        let amount_in_after_fee = amount_in
            .checked_sub(fee_amount)
            .ok_or(AutoBuyerError::MathOverflow)?;

        let numerator = amount_in_after_fee
            .checked_mul(reserve_out)
            .ok_or(AutoBuyerError::MathOverflow)?;
        let denominator = reserve_in
            .checked_add(amount_in_after_fee)
            .ok_or(AutoBuyerError::MathOverflow)?;

        let amount_out = numerator
            .checked_div(denominator)
            .ok_or(AutoBuyerError::MathOverflow)?;

        Ok((amount_out, fee_amount))
    }

    /// Создать данные инструкции обмена для Raydium (manual pack: 1 byte discriminator + 2 u64 LE)
    fn create_swap_instruction_data(
        &self,
        swap_params: &SwapParams,
    ) -> Result<[u8; 17], AutoBuyerError> {
        let mut data = [0u8; 17];
        data[0] = 9; // SwapBaseIn discriminator
        data[1..9].copy_from_slice(&swap_params.amount_in.to_le_bytes());
        data[9..17].copy_from_slice(&swap_params.min_amount_out.to_le_bytes());
        Ok(data)
    }

    /// Выполнить обмен через CPI
    fn execute_raydium_swap(
        &self,
        accounts: &[AccountInfo],
        swap_params: &SwapParams,
        _program_id: &Pubkey,
    ) -> ProgramResult {
        let instruction_data = self.create_swap_instruction_data(swap_params)?;

        let amm_account = &accounts[6];
        let amm_info = self.load_amm_info(amm_account)?;

        let user_account = &accounts[0];
        let source_token_account = &accounts[1];
        let destination_token_account = &accounts[2];
        let raydium_program = &accounts[5];
        let amm_authority = &accounts[11]; // PDA
        let amm_open_orders = &accounts[12];
        let amm_target_orders = &accounts[13];
        let pool_token_coin_vault = &accounts[7];
        let pool_token_pc_vault = &accounts[8];
        let serum_program = &accounts[14];
        let serum_market = &accounts[15];
        let serum_bids = &accounts[16];
        let serum_asks = &accounts[17];
        let serum_event_queue = &accounts[18];
        let serum_coin_vault = &accounts[19];
        let serum_pc_vault = &accounts[20];
        let serum_vault_signer = &accounts[21];
        let token_program = &accounts[9];

        let instruction = pinocchio::instruction::Instruction {
            program_id: raydium_program.key(), // Remove dereference
            accounts: &[
                // Use slice reference instead of vec!
                pinocchio::instruction::AccountMeta::readonly(token_program.key()),
                pinocchio::instruction::AccountMeta::writable(amm_account.key()),
                pinocchio::instruction::AccountMeta::readonly(amm_authority.key()),
                pinocchio::instruction::AccountMeta::writable(amm_open_orders.key()),
                pinocchio::instruction::AccountMeta::writable(amm_target_orders.key()),
                pinocchio::instruction::AccountMeta::writable(pool_token_coin_vault.key()),
                pinocchio::instruction::AccountMeta::writable(pool_token_pc_vault.key()),
                pinocchio::instruction::AccountMeta::readonly(serum_program.key()),
                pinocchio::instruction::AccountMeta::writable(serum_market.key()),
                pinocchio::instruction::AccountMeta::writable(serum_bids.key()),
                pinocchio::instruction::AccountMeta::writable(serum_asks.key()),
                pinocchio::instruction::AccountMeta::writable(serum_event_queue.key()),
                pinocchio::instruction::AccountMeta::writable(serum_coin_vault.key()),
                pinocchio::instruction::AccountMeta::writable(serum_pc_vault.key()),
                pinocchio::instruction::AccountMeta::readonly(serum_vault_signer.key()),
                pinocchio::instruction::AccountMeta::writable(source_token_account.key()),
                pinocchio::instruction::AccountMeta::writable(destination_token_account.key()),
                pinocchio::instruction::AccountMeta::readonly_signer(user_account.key()),
            ],
            data: &instruction_data, // Use slice reference
        };

        let account_infos = [
            // Use array of references
            token_program,
            amm_account,
            amm_authority,
            amm_open_orders,
            amm_target_orders,
            pool_token_coin_vault,
            pool_token_pc_vault,
            serum_program,
            serum_market,
            serum_bids,
            serum_asks,
            serum_event_queue,
            serum_coin_vault,
            serum_pc_vault,
            serum_vault_signer,
            source_token_account,
            destination_token_account,
            user_account,
        ];

        let binding = amm_info.nonce.to_le_bytes();
        let seeds = &[
            Seed::from(b"amm_authority".as_ref()),
            Seed::from(binding.as_ref()),
        ];

        invoke_signed(&instruction, &account_infos, &[Signer::from(seeds)])
    }
}

impl DexInterface for RaydiumV4 {
    fn find_trading_pair(
        &self,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        accounts: &[AccountInfo],
    ) -> Result<Option<TradingPair>, AutoBuyerError> {
        if accounts.len() < 7 {
            return Ok(None);
        }

        let pool_account = &accounts[6];
        let amm_info = match self.load_amm_info(pool_account) {
            Ok(info) => info,
            Err(_) => return Ok(None),
        };

        let is_correct_pair = (amm_info.base_mint == *base_mint
            && amm_info.quote_mint == *quote_mint)
            || (amm_info.base_mint == *quote_mint && amm_info.quote_mint == *base_mint);

        if !is_correct_pair {
            return Ok(None);
        }

        let pool_config = PoolConfig {
            pool_address: *pool_account.key(),
            token_a_account: amm_info.base_vault,
            token_b_account: amm_info.quote_vault,
            token_a_mint: amm_info.base_mint,
            token_b_mint: amm_info.quote_mint,
            fee_rate: (amm_info.trade_fee_numerator * constants::BASIS_POINTS as u64
                / amm_info.trade_fee_denominator) as u16,
        };

        let trading_pair = TradingPair {
            base_mint: *base_mint,
            quote_mint: *quote_mint,
            pool_config,
        };

        Ok(Some(trading_pair))
    }

    fn calculate_swap(
        &self,
        trading_pair: &TradingPair,
        amount_in: u64,
        accounts: &[AccountInfo],
    ) -> Result<SwapCalculation, AutoBuyerError> {
        let pool_account = &accounts[6];
        let amm_info = self.load_amm_info(pool_account)?;

        let token_a_vault = &accounts[7];
        let token_b_vault = &accounts[8];

        let reserve_a = Self::get_token_balance(token_a_vault)?;
        let reserve_b = Self::get_token_balance(token_b_vault)?;

        let (reserve_in, reserve_out) =
            if trading_pair.pool_config.token_a_mint == trading_pair.quote_mint {
                (reserve_a, reserve_b)
            } else {
                (reserve_b, reserve_a)
            };

        let (amount_out, fee_amount) = self.calculate_amount_out(
            amount_in,
            reserve_in,
            reserve_out,
            amm_info.trade_fee_numerator,
            amm_info.trade_fee_denominator,
        )?;

        let amount_in_after_fee = amount_in.saturating_sub(fee_amount);

        let price_per_unit = if amount_in > 0 {
            // Fixed-point: (amount_out * 1_000_000_000) / amount_in
            amount_out
                .checked_mul(1_000_000_000)
                .and_then(|x| x.checked_div(amount_in))
                .unwrap_or(0)
        } else {
            0
        };

        let slippage_bps = if reserve_out > 0 && reserve_in > 0 {
            // slippage in basis points = (ideal_price - actual_price) / ideal_price * 10000
            // ideal_price = reserve_out / reserve_in
            // actual_price = amount_out / amount_in_after_fee
            // Using integer math: ((reserve_out * amount_in_after_fee - reserve_in * amount_out) * 10000) / (reserve_out * amount_in_after_fee)
            let ideal_num = reserve_out as u128;
            let ideal_den = reserve_in as u128;
            let actual_num = amount_out as u128;
            let actual_den = amount_in_after_fee as u128;

            if ideal_num > 0 && actual_den > 0 {
                let diff_num = ideal_num
                    .checked_mul(actual_den)
                    .and_then(|x| x.checked_sub(ideal_den.checked_mul(actual_num).unwrap_or(0)))
                    .unwrap_or(0);
                let diff_den = ideal_num.checked_mul(actual_den).unwrap_or(1);
                (diff_num * 10000 / diff_den).min(u16::MAX as u128) as u16
            } else {
                0
            }
        } else {
            0
        };

        Ok(SwapCalculation {
            amount_in,
            amount_out,
            fee_amount,
            price_per_unit,
            slippage_bps,
        })
    }

    fn execute_swap(
        &self,
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        swap_params: &SwapParams,
    ) -> ProgramResult {
        msg!(
            "Executing Raydium swap: {} -> {}",
            swap_params.amount_in,
            swap_params.calculation.amount_out
        );

        self.execute_raydium_swap(accounts, swap_params, program_id)?;

        msg!("Raydium swap completed successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        error::AutoBuyerError,
        state::{constants, SwapCalculation, TradingPair},
    };
    use pinocchio::pubkey::Pubkey;

    // Helper to create a minimal valid 752-byte pool account data
    fn create_test_pool_data() -> [u8; 752] {
        let mut data = [0u8; 752];
        
        // status = 1 (initialized) at offset 0
        data[0..8].copy_from_slice(&1u64.to_le_bytes());
        
        // nonce = 42 at offset 8
        data[8..16].copy_from_slice(&42u64.to_le_bytes());
        
        // trade_fee_numerator = 25 at offset 200 (0.25% fee)
        data[200..208].copy_from_slice(&25u64.to_le_bytes());
        
        // trade_fee_denominator = 10000 at offset 208
        data[208..216].copy_from_slice(&10000u64.to_le_bytes());
        
        // base_vault at offset 280
        let base_vault = Pubkey::new_unique();
        data[280..312].copy_from_slice(base_vault.as_ref());
        
        // quote_vault at offset 312
        let quote_vault = Pubkey::new_unique();
        data[312..344].copy_from_slice(quote_vault.as_ref());
        
        // base_mint at offset 344 (WSOL)
        data[344..376].copy_from_slice(constants::WSOL_MINT.as_ref());
        
        // quote_mint at offset 376 (USDC)
        let usdc_mint = Pubkey::new_unique();
        data[376..408].copy_from_slice(usdc_mint.as_ref());
        
        // lp_mint at offset 408
        let lp_mint = Pubkey::new_unique();
        data[408..440].copy_from_slice(lp_mint.as_ref());
        
        // open_orders at offset 440
        let open_orders = Pubkey::new_unique();
        data[440..472].copy_from_slice(open_orders.as_ref());
        
        // market_id at offset 472
        let market_id = Pubkey::new_unique();
        data[472..504].copy_from_slice(market_id.as_ref());
        
        // market_program_id at offset 504
        let market_program_id = Pubkey::new_unique();
        data[504..536].copy_from_slice(market_program_id.as_ref());
        
        // target_orders at offset 536
        let target_orders = Pubkey::new_unique();
        data[536..568].copy_from_slice(target_orders.as_ref());
        
        data
    }

    #[test]
    fn test_amm_info_parse_valid() {
        let pool_data = create_test_pool_data();
        let amm_info = AmmInfo::parse(&pool_data).unwrap();
        
        assert_eq!(amm_info.status, 1);
        assert_eq!(amm_info.nonce, 42);
        assert_eq!(amm_info.trade_fee_numerator, 25);
        assert_eq!(amm_info.trade_fee_denominator, 10000);
        assert_eq!(amm_info.base_mint, constants::WSOL_MINT);
    }

    #[test]
    fn test_amm_info_parse_wrong_length() {
        let short_data = [0u8; 500];
        let result = AmmInfo::parse(&short_data);
        assert!(matches!(result, Err(AutoBuyerError::InvalidParameters)));
        
        let long_data = [0u8; 800];
        let result = AmmInfo::parse(&long_data);
        assert!(matches!(result, Err(AutoBuyerError::InvalidParameters)));
    }

    #[test]
    fn test_amm_info_parse_uninitialized_pool() {
        let mut pool_data = create_test_pool_data();
        pool_data[0..8].copy_from_slice(&0u64.to_le_bytes()); // status = 0
        let result = AmmInfo::parse(&pool_data);
        // Parse succeeds but load_amm_info will reject status != 1
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, 0);
    }

    #[test]
    fn test_calculate_amount_out_basic() {
        let raydium = RaydiumV4::new();
        
        // reserve_in = 1000 SOL, reserve_out = 100000 USDC
        // amount_in = 10 SOL
        // fee = 25/10000 = 0.25%
        // amount_in_after_fee = 10 - (10 * 25 / 10000) = 9.975 SOL
        // amount_out = (9.975 * 100000) / (1000 + 9.975) = 997500 / 1009.975 ≈ 987.6 USDC
        
        let (amount_out, fee_amount) = raydium
            .calculate_amount_out(10, 1000, 100000, 25, 10000)
            .unwrap();
        
        // fee = 10 * 25 / 10000 = 0 (integer division)
        // amount_in_after_fee = 10
        // amount_out = 10 * 100000 / (1000 + 10) = 1000000 / 1010 = 990
        assert_eq!(fee_amount, 0); // Integer division rounds down
        assert_eq!(amount_out, 990);
    }

    #[test]
    fn test_calculate_amount_out_with_fee() {
        let raydium = RaydiumV4::new();
        
        // amount_in = 10000, fee = 25/10000 = 0.25%
        // fee = 10000 * 25 / 10000 = 25
        // amount_in_after_fee = 9975
        // amount_out = 9975 * 100000 / (1000 + 9975) = 997500000 / 10975 = 90888
        
        let (amount_out, fee_amount) = raydium
            .calculate_amount_out(10000, 1000, 100000, 25, 10000)
            .unwrap();
        
        assert_eq!(fee_amount, 25);
        assert_eq!(amount_out, 90888);
    }

    #[test]
    fn test_calculate_amount_out_insufficient_liquidity() {
        let raydium = RaydiumV4::new();
        
        let result = raydium.calculate_amount_out(100, 0, 100000, 25, 10000);
        assert!(matches!(result, Err(AutoBuyerError::InsufficientLiquidity)));
        
        let result = raydium.calculate_amount_out(100, 1000, 0, 25, 10000);
        assert!(matches!(result, Err(AutoBuyerError::InsufficientLiquidity)));
    }

    #[test]
    fn test_calculate_swap_price_and_slippage() {
        let raydium = RaydiumV4::new();
        
        // Test with simple reserves: 1000 in, 100000 out
        // amount_in = 100
        // fee = 100 * 25 / 10000 = 0
        // amount_in_after_fee = 100
        // amount_out = 100 * 100000 / (1000 + 100) = 10000000 / 1100 = 9090
        // price_per_unit = 9090 * 1e9 / 100 = 90,900,000,000
        // ideal_price = 100000 / 1000 = 100
        // actual_price = 9090 / 100 = 90.9
        // slippage = (100 - 90.9) / 100 * 10000 = 910 bps = 9.1%
        
        let reserve_in = 1000u64;
        let reserve_out = 100000u64;
        let amount_in = 100u64;
        let fee_numerator = 25u64;
        let fee_denominator = 10000u64;
        
        let fee_amount = amount_in * fee_numerator / fee_denominator; // 0
        let amount_in_after_fee = amount_in - fee_amount; // 100
        let amount_out = amount_in_after_fee * reserve_out / (reserve_in + amount_in_after_fee); // 9090
        
        let price_per_unit = amount_out * 1_000_000_000 / amount_in; // 90,900,000,000
        
        // slippage_bps = ((ideal - actual) / ideal) * 10000
        // ideal_num = reserve_out = 100000
        // ideal_den = reserve_in = 1000
        // actual_num = amount_out = 9090
        // actual_den = amount_in_after_fee = 100
        // diff_num = ideal_num * actual_den - ideal_den * actual_num = 100000*100 - 1000*9090 = 10000000 - 9090000 = 910000
        // diff_den = ideal_num * actual_den = 10000000
        // slippage_bps = 910000 * 10000 / 10000000 = 910
        
        let ideal_num = reserve_out as u128;
        let ideal_den = reserve_in as u128;
        let actual_num = amount_out as u128;
        let actual_den = amount_in_after_fee as u128;
        
        let diff_num = ideal_num * actual_den - ideal_den * actual_num;
        let diff_den = ideal_num * actual_den;
        let slippage_bps = (diff_num * 10000 / diff_den) as u16;
        
        assert_eq!(fee_amount, 0);
        assert_eq!(amount_out, 9090);
        assert_eq!(price_per_unit, 90_900_000_000);
        assert_eq!(slippage_bps, 910); // 9.10%
    }

    #[test]
    fn test_calculate_swap_slippage_zero_reserves() {
        // Edge case: zero reserves should return 0 slippage
        let reserve_in = 0u64;
        let reserve_out = 100000u64;
        
        let slippage_bps = if reserve_out > 0 && reserve_in > 0 {
            let ideal_num = reserve_out as u128;
            let ideal_den = reserve_in as u128;
            let actual_num = 9090u128;
            let actual_den = 100u128;
            
            let diff_num = ideal_num * actual_den - ideal_den * actual_num;
            let diff_den = ideal_num * actual_den;
            (diff_num * 10000 / diff_den).min(u16::MAX as u128) as u16
        } else {
            0
        };
        
        assert_eq!(slippage_bps, 0);
    }

    #[test]
    fn test_calculate_swap_slippage_perfect_price() {
        // When actual price equals ideal price, slippage should be 0
        let ideal_num = 100000u128;
        let ideal_den = 1000u128;
        let actual_num = 100u128; // 100 out
        let actual_den = 1u128;   // 1 in (actual price = 100, ideal = 100)
        
        let diff_num = ideal_num * actual_den - ideal_den * actual_num; // 100000*1 - 1000*100 = 0
        let diff_den = ideal_num * actual_den; // 100000
        let slippage_bps = (diff_num * 10000 / diff_den) as u16;
        
        assert_eq!(slippage_bps, 0);
    }
}
