use pinocchio::{
    account_info::AccountInfo, entrypoint::ProgramResult, instruction::Seed, instruction::Signer,
    msg, program::invoke_signed, pubkey::Pubkey,
};

use spl_token::solana_program::program_pack::Pack;

use borsh::{BorshDeserialize, BorshSerialize};
use spl_token::state::Account as TokenAccount;

use crate::{
    dex::{
        types::{DexProvider, SwapParams},
        DexInterface,
    },
    error::AutoBuyerError,
    state::{constants, PoolConfig, SwapCalculation, TradingPair},
};

/// Структура для работы с Raydium v4
pub struct RaydiumV4;

/// Инструкция обмена Raydium
#[derive(BorshSerialize, BorshDeserialize)]
struct RaydiumSwapInstruction {
    instruction: u8, // 9 для swap
    amount_in: u64,
    minimum_amount_out: u64,
}

/// Состояние AMM пула Raydium v4
#[derive(BorshDeserialize, Debug)]
#[repr(C)]
pub struct AmmInfo {
    pub status: u64,
    pub nonce: u64,
    pub order_num: u64,
    pub depth: u64,
    pub base_decimals: u64,
    pub quote_decimals: u64,
    pub state: u64,
    pub reset_flag: u64,
    pub min_size: u64,
    pub vol_max_cut_ratio: u64,
    pub amount_wave_ratio: u64,
    pub base_lot_size: u64,
    pub quote_lot_size: u64,
    pub min_price_multiplier: u64,
    pub max_price_multiplier: u64,
    pub system_decimals_value: u64,
    pub min_separate_numerator: u64,
    pub min_separate_denominator: u64,
    pub trade_fee_numerator: u64,
    pub trade_fee_denominator: u64,
    pub pnl_numerator: u64,
    pub pnl_denominator: u64,
    pub swap_fee_numerator: u64,
    pub swap_fee_denominator: u64,
    pub base_need_take_pnl: u64,
    pub quote_need_take_pnl: u64,
    pub quote_total_pnl: u64,
    pub base_total_pnl: u64,
    pub pool_open_time: u64,
    pub punish_pc_amount: u64,
    pub punish_coin_amount: u64,
    pub orderbook_to_init_time: u64,
    pub swap_base_in_amount: u128,
    pub swap_quote_out_amount: u128,
    pub swap_base2_quote_fee: u64,
    pub swap_quote_in_amount: u128,
    pub swap_base_out_amount: u128,
    pub swap_quote2_base_fee: u64,
    pub base_vault: Pubkey,
    pub quote_vault: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub open_orders: Pubkey,
    pub market_id: Pubkey,
    pub market_program_id: Pubkey,
    pub target_orders: Pubkey,
    pub withdraw_queue: Pubkey,
    pub lp_vault: Pubkey,
    pub owner: Pubkey,
    pub lp_reserve: u64,
}

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

        AmmInfo::try_from_slice(&pool_data).map_err(|_| AutoBuyerError::InvalidParameters)
    }

    /// Получить баланс токенов из аккаунта
    fn get_token_balance(account_info: &AccountInfo) -> Result<u64, AutoBuyerError> {
        let token_account = TokenAccount::unpack(
            &account_info
                .try_borrow_data()
                .map_err(|_| AutoBuyerError::CpiError)?,
        )
        .map_err(|e| {
            msg!("Token error: {:?}", e);
            AutoBuyerError::CpiError
        })?;
        Ok(token_account.amount)
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

    /// Создать данные инструкции обмена для Raydium
    fn create_swap_instruction_data(
        &self,
        swap_params: &SwapParams,
    ) -> Result<Vec<u8>, AutoBuyerError> {
        let instruction_data = RaydiumSwapInstruction {
            instruction: 9, // Raydium swap instruction
            amount_in: swap_params.amount_in,
            minimum_amount_out: swap_params.min_amount_out,
        };
        borsh::to_vec(&instruction_data).map_err(|_| AutoBuyerError::InvalidParameters)
    }

    /// Выполнить обмен через CPI
    fn execute_raydium_swap(
        &self,
        accounts: &[AccountInfo],
        swap_params: &SwapParams,
        program_id: &Pubkey,
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

        let price_per_unit = if amount_in > 0 {
            amount_out as f64 / amount_in as f64
        } else {
            0.0
        };

        let slippage_percent = if reserve_out > 0 {
            let ideal_price = reserve_out as f64 / reserve_in as f64;
            ((ideal_price - price_per_unit) / ideal_price * 100.0).abs()
        } else {
            0.0
        };

        Ok(SwapCalculation {
            amount_in,
            amount_out,
            fee_amount,
            price_per_unit,
            slippage_percent,
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

    fn provider_type(&self) -> DexProvider {
        DexProvider::RaydiumV4
    }
}
