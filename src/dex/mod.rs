pub mod raydium;
pub mod types;

use pinocchio::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::{
    error::AutoBuyerError,
    state::{SwapCalculation, TradingPair},
};

use self::types::SwapParams;

/// Trait for interacting with DEX
pub trait DexInterface {
    /// Find trading pair
    fn find_trading_pair(
        &self,
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        accounts: &[AccountInfo],
    ) -> Result<Option<TradingPair>, AutoBuyerError>;

    /// Calculate swap
    fn calculate_swap(
        &self,
        trading_pair: &TradingPair,
        amount_in: u64,
        accounts: &[AccountInfo],
    ) -> Result<SwapCalculation, AutoBuyerError>;

    /// Execute swap
    fn execute_swap(
        &self,
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        swap_params: &SwapParams,
    ) -> ProgramResult;
}

/// DEX manager for selecting appropriate provider
pub struct DexManager {
    providers: Vec<Box<dyn DexInterface>>,
}

impl Default for DexManager {
    fn default() -> Self {
        let providers: Vec<Box<dyn DexInterface>> = vec![Box::new(raydium::RaydiumV4)];
        Self { providers }
    }
}

impl DexManager {
    /// Create new DEX manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Find best trading pair among all DEX providers
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

    /// Execute automatic swap
    pub fn execute_auto_swap(
        &self,
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        base_mint: &Pubkey,
        quote_mint: &Pubkey,
        amount_in: u64,
        min_amount_out: u64,
    ) -> Result<SwapCalculation, AutoBuyerError> {
        // Find best trading pair
        let (trading_pair, provider) =
            self.find_best_trading_pair(base_mint, quote_mint, accounts)?;

        // Calculate swap
        let calculation = provider.calculate_swap(&trading_pair, amount_in, accounts)?;

        // Check slippage
        if calculation.amount_out < min_amount_out {
            return Err(AutoBuyerError::SlippageTooHigh);
        }

        // Execute swap
        let swap_params = SwapParams {
            trading_pair,
            amount_in,
            min_amount_out,
            calculation,
        };

        provider
            .execute_swap(program_id, accounts, &swap_params)
            .map_err(|_| AutoBuyerError::CpiError)?;

        Ok(calculation)
    }
}
