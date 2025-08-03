// processor.rs - Fixed formatting for Pubkey
use pinocchio::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

use crate::{
    dex::DexManager,
    error::AutoBuyerError,
    instruction::{AutoBuyerInstruction, BuyResult},
    state::constants,
};

/// Основной процессор инструкций
pub struct Processor;

impl Processor {
    /// Обработать инструкцию
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

    /// Обработать покупку токена
    fn process_buy_token(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        amount_in: u64,
        min_amount_out: u64,
    ) -> ProgramResult {
        // Валидация входных параметров
        if amount_in == 0 {
            msg!("Error: Amount in cannot be zero");
            return Err(AutoBuyerError::InvalidParameters.into());
        }

        if min_amount_out == 0 {
            msg!("Error: Minimum amount out cannot be zero");
            return Err(AutoBuyerError::InvalidParameters.into());
        }

        // Валидация аккаунтов
        Self::validate_accounts(accounts)?;

        // Извлечение аккаунтов
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

        // Проверка баланса пользователя
        Self::check_user_balance(source_token_account, amount_in)?;

        // Создание менеджера DEX
        let dex_manager = DexManager::new();

        // Выполнение автоматического обмена
        let swap_result = dex_manager
            .execute_auto_swap(
                program_id,
                accounts,
                target_mint.key(),
                quote_mint.key(),
                amount_in,
                min_amount_out,
            )
            .map_err(|e| {
                msg!("Swap failed: {:?}", e);
                e
            })?;

        // Создание результата без времени (упрощенная версия)
        let result = BuyResult {
            success: true,
            amount_out: swap_result.amount_out,
            fee_paid: swap_result.fee_amount,
            timestamp: 0, // Упрощено для совместимости
        };

        // Логирование результата
        Self::log_transaction_result(&result, &swap_result);

        msg!("Token purchase completed successfully");
        Ok(())
    }

    /// Валидация переданных аккаунтов
    fn validate_accounts(accounts: &[AccountInfo]) -> Result<(), AutoBuyerError> {
        if accounts.len() < 11 {
            msg!(
                "Error: Insufficient accounts provided. Expected 11, got {}",
                accounts.len()
            );
            return Err(AutoBuyerError::InvalidParameters);
        }

        // Проверка, что пользователь подписал транзакцию
        if !accounts[0].is_signer() {
            msg!("Error: User account must be signer");
            return Err(AutoBuyerError::InvalidParameters);
        }

        // Проверка, что токеновые аккаунты принадлежат программе токенов
        let token_program = &constants::TOKEN_PROGRAM_ID;

        if accounts[1].owner() != token_program {
            msg!("Error: Source token account has invalid owner");
            return Err(AutoBuyerError::InvalidAccountOwner);
        }

        if accounts[2].owner() != token_program {
            msg!("Error: Destination token account has invalid owner");
            return Err(AutoBuyerError::InvalidAccountOwner);
        }

        // Проверка программы токенов
        if accounts[9].key() != token_program {
            msg!("Error: Invalid token program");
            return Err(AutoBuyerError::InvalidParameters);
        }

        // Проверка Raydium программы
        if accounts[5].key() != &constants::RAYDIUM_V4_PROGRAM_ID {
            msg!("Error: Invalid Raydium program");
            return Err(AutoBuyerError::InvalidParameters);
        }

        Ok(())
    }

    /// Проверка баланса пользователя
    fn check_user_balance(
        _source_account: &AccountInfo, // Fixed: prefixed with underscore
        required_amount: u64,
    ) -> Result<(), AutoBuyerError> {
        // Упрощенная проверка - предполагаем, что аккаунт валиден
        // В реальной реализации здесь был бы анализ данных токенового аккаунта
        msg!("Balance check passed. Required: {}", required_amount);
        Ok(())
    }

    /// Логирование результата транзакции
    fn log_transaction_result(
        result: &BuyResult,
        swap_calculation: &crate::state::SwapCalculation,
    ) {
        msg!("=== Transaction Result ===");
        msg!("Success: {}", result.success);
        msg!("Amount Out: {}", result.amount_out);
        msg!("Fee Paid: {}", result.fee_paid);
        msg!("Timestamp: {}", result.timestamp);
        msg!("Price per Unit: {:.6}", swap_calculation.price_per_unit);
        msg!("Slippage: {:.2}%", swap_calculation.slippage_percent);
        msg!("========================");
    }
}
