pub mod raydium;
pub mod types;

use pinocchio::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::{
    error::AutoBuyerError,
    state::{SwapCalculation, TradingPair},
};

use self::types::SwapParams;

/// Трейт для взаимодействия с DEX
pub trait DexInterface {
    /// Найти торговую пару
    fn find_trading_pair(
        &self,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        accounts: &[AccountInfo],
    ) -> Result<Option<TradingPair>, AutoBuyerError>;

    /// Рассчитать обмен
    fn calculate_swap(
        &self,
        trading_pair: &TradingPair,
        amount_in: u64,
        accounts: &[AccountInfo],
    ) -> Result<SwapCalculation, AutoBuyerError>;

    /// Выполнить обмен
    fn execute_swap(
        &self,
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        swap_params: &SwapParams,
    ) -> ProgramResult;
}

/// Менеджер DEX для выбора подходящего провайдера
pub struct DexManager {
    providers: Vec<Box<dyn DexInterface>>,
}

impl DexManager {
    /// Создать новый менеджер DEX
    pub fn new() -> Self {
        let providers: Vec<Box<dyn DexInterface>> = vec![Box::new(raydium::RaydiumV4::new())];

        Self { providers }
    }

    /// Найти лучшую торговую пару среди всех DEX
    pub fn find_best_trading_pair(
        &self,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        accounts: &[AccountInfo],
    ) -> Result<(TradingPair, &dyn DexInterface), AutoBuyerError> {
        for provider in &self.providers {
            if let Some(trading_pair) =
                provider.find_trading_pair(base_mint, quote_mint, accounts)?
            {
                return Ok((trading_pair, provider.as_ref()));
            }
        }

        Err(AutoBuyerError::PoolNotFound)
    }

    /// Выполнить автоматический обмен
    pub fn execute_auto_swap(
        &self,
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<SwapCalculation, AutoBuyerError> {
        // Найти лучшую торговую пару
        let (trading_pair, provider) =
            self.find_best_trading_pair(base_mint, quote_mint, accounts)?;

        // Рассчитать обмен
        let calculation = provider.calculate_swap(&trading_pair, amount_in, accounts)?;

        // Проверить проскальзывание
        if calculation.amount_out < min_amount_out {
            return Err(AutoBuyerError::SlippageTooHigh);
        }

        // Выполнить обмен
        let swap_params = SwapParams {
            trading_pair,
            amount_in,
            min_amount_out,
            calculation: calculation.clone(),
        };

        provider
            .execute_swap(program_id, accounts, &swap_params)
            .map_err(|_| AutoBuyerError::CpiError)?;

        Ok(calculation)
    }
}
