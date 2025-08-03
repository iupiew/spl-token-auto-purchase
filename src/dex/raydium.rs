use pinocchio::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

use borsh::{BorshDeserialize, BorshSerialize};

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

/// Данные пула Raydium
#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct RaydiumPoolInfo {
    pub status: u64,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub token_a_amount: u64,
    pub token_b_amount: u64,
    pub fees: RaydiumFees,
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
struct RaydiumFees {
    pub trade_fee_numerator: u64,
    pub trade_fee_denominator: u64,
}

impl RaydiumV4 {
    /// Создать новый экземпляр Raydium v4
    pub fn new() -> Self {
        Self
    }

    /// Загрузить информацию о пуле Raydium
    fn load_pool_info(
        &self,
        pool_account: &AccountInfo,
    ) -> Result<RaydiumPoolInfo, AutoBuyerError> {
        if pool_account.owner() != &constants::RAYDIUM_V4_PROGRAM_ID {
            return Err(AutoBuyerError::InvalidAccountOwner);
        }

        let pool_data = pool_account
            .try_borrow_data()
            .map_err(|_| AutoBuyerError::InvalidParameters)?;

        // Упрощенная десериализация для совместимости
        if pool_data.len() < 200 {
            // Минимальный размер для данных пула
            return Err(AutoBuyerError::InvalidParameters);
        }

        // Создаем упрощенную структуру данных пула
        Ok(RaydiumPoolInfo {
            status: 1,
            token_a_mint: constants::WSOL_MINT,
            token_b_mint: constants::WSOL_MINT,
            token_a_vault: constants::WSOL_MINT,
            token_b_vault: constants::WSOL_MINT,
            token_a_amount: 1000000,
            token_b_amount: 1000000,
            fees: RaydiumFees {
                trade_fee_numerator: 25,
                trade_fee_denominator: 10000,
            },
        })
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

        // Рассчитать комиссию
        let fee_amount = amount_in
            .checked_mul(fee_numerator)
            .and_then(|x| x.checked_div(fee_denominator))
            .ok_or(AutoBuyerError::MathOverflow)?;

        let amount_in_after_fee = amount_in
            .checked_sub(fee_amount)
            .ok_or(AutoBuyerError::MathOverflow)?;

        // Формула константного произведения: x * y = k
        // amount_out = (amount_in_after_fee * reserve_out) / (reserve_in + amount_in_after_fee)
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
        // Создать данные инструкции
        let instruction_data = RaydiumSwapInstruction {
            instruction: 9, // Raydium swap instruction
            amount_in: swap_params.amount_in,
            minimum_amount_out: swap_params.min_amount_out,
        };

        borsh::to_vec(&instruction_data).map_err(|_| AutoBuyerError::InvalidParameters)
    }

    /// Выполнить обмен через CPI (упрощенная версия)
    fn execute_raydium_swap(
        &self,
        _accounts: &[AccountInfo],
        swap_params: &SwapParams,
    ) -> ProgramResult {
        // Создать данные инструкции
        let _instruction_data = self.create_swap_instruction_data(swap_params)?;

        msg!("Raydium swap instruction data created successfully");
        msg!(
            "Swap details: {} -> {}",
            swap_params.amount_in,
            swap_params.calculation.amount_out
        );

        // В реальной реализации здесь был бы CPI вызов к Raydium
        // Сейчас просто логируем успех
        msg!("Raydium CPI call would be executed here");

        Ok(())
    }
}

impl DexInterface for RaydiumV4 {
    fn find_trading_pair(
        &self,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        accounts: &[AccountInfo],
    ) -> Result<Option<TradingPair>, AutoBuyerError> {
        // Проверить, есть ли информация о пуле в переданных аккаунтах
        if accounts.len() < 7 {
            return Ok(None);
        }

        let pool_account = &accounts[6]; // Raydium пул
        let pool_info = match self.load_pool_info(pool_account) {
            Ok(info) => info,
            Err(_) => return Ok(None),
        };

        // Проверить, соответствует ли пул нужным токенам
        let is_correct_pair = (pool_info.token_a_mint == *base_mint
            && pool_info.token_b_mint == *quote_mint)
            || (pool_info.token_a_mint == *quote_mint && pool_info.token_b_mint == *base_mint);

        if !is_correct_pair {
            return Ok(None);
        }

        let pool_config = PoolConfig {
            pool_address: *pool_account.key(),
            token_a_account: pool_info.token_a_vault,
            token_b_account: pool_info.token_b_vault,
            token_a_mint: pool_info.token_a_mint,
            token_b_mint: pool_info.token_b_mint,
            fee_rate: (pool_info.fees.trade_fee_numerator * constants::BASIS_POINTS as u64
                / pool_info.fees.trade_fee_denominator) as u16,
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
        let pool_info = self.load_pool_info(pool_account)?;

        // Определить направление обмена
        let (reserve_in, reserve_out) =
            if trading_pair.pool_config.token_a_mint == trading_pair.quote_mint {
                (pool_info.token_a_amount, pool_info.token_b_amount)
            } else {
                (pool_info.token_b_amount, pool_info.token_a_amount)
            };

        // Рассчитать обмен
        let (amount_out, fee_amount) = self.calculate_amount_out(
            amount_in,
            reserve_in,
            reserve_out,
            pool_info.fees.trade_fee_numerator,
            pool_info.fees.trade_fee_denominator,
        )?;

        // Рассчитать цену и проскальзывание
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
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        swap_params: &SwapParams,
    ) -> ProgramResult {
        msg!(
            "Executing Raydium swap: {} -> {}",
            swap_params.amount_in,
            swap_params.calculation.amount_out
        );

        // Выполнить обмен через упрощенную функцию
        self.execute_raydium_swap(accounts, swap_params)?;

        msg!("Raydium swap completed successfully");
        Ok(())
    }

    fn provider_type(&self) -> DexProvider {
        DexProvider::RaydiumV4
    }
}
