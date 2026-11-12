use pinocchio::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

use crate::{
    dex::DexManager,
    error::AutoBuyerError,
    instruction::{AutoBuyerInstruction, BuyResult},
    state::constants,
};

/// Main instruction processor
pub struct Processor;

impl Processor {
    /// Process instruction
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction: AutoBuyerInstruction,
    ) -> ProgramResult {
        match instruction {
            AutoBuyerInstruction::BuyToken {
                amount_in,
                min_amount_out,
            } => {
                msg!("Processing BuyToken instruction");
                Self::process_buy_token(program_id, accounts, amount_in, min_amount_out)
            }
        }
    }

    /// Process token purchase
    fn process_buy_token(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount_in: u64,
        min_amount_out: u64,
    ) -> ProgramResult {
        // Validate input parameters
        if amount_in == 0 {
            msg!("Error: Amount in cannot be zero");
            return Err(AutoBuyerError::InvalidParameters.into());
        }

        if min_amount_out == 0 {
            msg!("Error: Minimum amount out cannot be zero");
            return Err(AutoBuyerError::InvalidParameters.into());
        }

        // Validate accounts
        Self::validate_accounts(accounts)?;

        // Extract accounts
        let user_account = &accounts[0];
        let source_token_account = &accounts[1];
        let _destination_token_account = &accounts[2]; // Fixed: prefixed with underscore
        let target_mint = &accounts[3];
        let quote_mint = &accounts[4];

        // Fixed: Format Pubkey as debug instead of display
        msg!("User: {:?}", user_account.key());
        msg!("Target mint: {:?}", target_mint.key());
        msg!("Quote mint: {:?}", quote_mint.key());
        msg!("Amount in: {}", amount_in);
        msg!("Min amount out: {}", min_amount_out);

        // Check user balance
        Self::check_user_balance(
            source_token_account,
            amount_in,
            quote_mint.key(),
            user_account.key(),
        )?;

        // Create DEX manager
        let dex_manager = DexManager::new();

        // Execute automatic swap
        let swap_result = dex_manager
            .execute_auto_swap(
                program_id,
                accounts,
                target_mint.key(),
                quote_mint.key(),
                amount_in,
                min_amount_out,
            )
            .inspect_err(|&e| {
                msg!("Swap failed: {:?}", e);
            })?;

        // Create result without timestamp (simplified for compatibility)
        let result = BuyResult {
            success: true,
            amount_out: swap_result.amount_out,
            fee_paid: swap_result.fee_amount,
            timestamp: 0, // Simplified for compatibility
        };

        // Log transaction result
        Self::log_transaction_result(&result, &swap_result);

        msg!("Token purchase completed successfully");
        Ok(())
    }

    /// Validate provided accounts
    fn validate_accounts(accounts: &[AccountInfo]) -> Result<(), AutoBuyerError> {
        if accounts.len() < 22 {
            msg!(
                "Error: Insufficient accounts provided. Expected 22, got {}",
                accounts.len()
            );
            return Err(AutoBuyerError::InvalidParameters);
        }

        // Check that user signed the transaction
        if !accounts[0].is_signer() {
            msg!("Error: User account must be signer");
            return Err(AutoBuyerError::InvalidParameters);
        }

        // Check that token accounts belong to token program
        let token_program = &constants::TOKEN_PROGRAM_ID;

        if accounts[1].owner() != token_program {
            msg!("Error: Source token account has invalid owner");
            return Err(AutoBuyerError::InvalidAccountOwner);
        }

        if accounts[2].owner() != token_program {
            msg!("Error: Destination token account has invalid owner");
            return Err(AutoBuyerError::InvalidAccountOwner);
        }

        // Verify token program
        if accounts[9].key() != token_program {
            msg!("Error: Invalid token program");
            return Err(AutoBuyerError::InvalidParameters);
        }

        // Verify Raydium program
        if accounts[5].key() != &constants::RAYDIUM_V4_PROGRAM_ID {
            msg!("Error: Invalid Raydium program");
            return Err(AutoBuyerError::InvalidParameters);
        }

        Ok(())
    }

    /// Check user token balance
    fn check_user_balance(
        source_account: &AccountInfo,
        required_amount: u64,
        expected_mint: &Pubkey,
        expected_owner: &Pubkey,
    ) -> Result<(), AutoBuyerError> {
        let data = source_account
            .try_borrow_data()
            .map_err(|_| AutoBuyerError::CpiError)?;

        // pinocchio_token::state::Account layout:
        // mint (32) + owner (32) + amount (8) + ...
        if data.len() < 72 {
            return Err(AutoBuyerError::InvalidParameters);
        }

        // Verify mint matches expected quote mint
        let mint_bytes: [u8; 32] = data[0..32].try_into().unwrap();
        if Pubkey::from(mint_bytes) != *expected_mint {
            msg!("Source account mint mismatch");
            return Err(AutoBuyerError::InvalidParameters);
        }

        // Verify owner/authority matches signer
        let owner_bytes: [u8; 32] = data[32..64].try_into().unwrap();
        if Pubkey::from(owner_bytes) != *expected_owner {
            msg!("Source account owner mismatch");
            return Err(AutoBuyerError::InvalidAccountOwner);
        }

        // Check amount
        let amount_bytes: [u8; 8] = data[64..72].try_into().unwrap();
        let amount = u64::from_le_bytes(amount_bytes);

        if amount < required_amount {
            msg!(
                "Insufficient balance: have {}, need {}",
                amount,
                required_amount
            );
            return Err(AutoBuyerError::InsufficientFunds);
        }

        Ok(())
    }

    /// Log transaction result
    fn log_transaction_result(
        result: &BuyResult,
        swap_calculation: &crate::state::SwapCalculation,
    ) {
        msg!("=== Transaction Result ===");
        msg!("Success: {}", result.success);
        msg!("Amount Out: {}", result.amount_out);
        msg!("Fee Paid: {}", result.fee_paid);
        msg!("Timestamp: {}", result.timestamp);
        // price_per_unit is fixed-point scaled by 1e9
        msg!(
            "Price per Unit: {}.{:09}",
            swap_calculation.price_per_unit / 1_000_000_000,
            swap_calculation.price_per_unit % 1_000_000_000
        );
        // slippage_bps is in basis points (10000 = 100%)
        msg!(
            "Slippage: {}.{:02}%",
            swap_calculation.slippage_bps / 100,
            swap_calculation.slippage_bps % 100
        );
        msg!("========================");
    }
}
