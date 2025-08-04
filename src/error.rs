use pinocchio::program_error::ProgramError;
use thiserror::Error;

/// Перечисление ошибок
#[derive(Error, Debug, Copy, Clone)]
pub enum AutoBuyerError {
    /// Неверная инструкция
    #[error("Invalid instruction")]
    InvalidInstruction,

    /// Недостаточные средства
    #[error("Insufficient funds")]
    InsufficientFunds,

    /// Недостаточная ликвидность
    #[error("Insufficient liquidity")]
    InsufficientLiquidity,

    /// Проскальзывание слишком велико
    #[error("Slippage too high")]
    SlippageTooHigh,

    /// Пул не найден
    #[error("Pool not found")]
    PoolNotFound,

    /// Неверные параметры
    #[error("Invalid parameters")]
    InvalidParameters,

    /// Неверный владелец аккаунта
    #[error("Invalid account owner")]
    InvalidAccountOwner,

    /// Математическое переполнение
    #[error("Math overflow")]
    MathOverflow,

    /// Ошибка CPI
    #[error("CPI error")]
    CpiError,

    /// Токен не поддерживается
    #[error("Token not supported")]
    TokenNotSupported,
}

impl From<AutoBuyerError> for ProgramError {
    fn from(e: AutoBuyerError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl From<ProgramError> for AutoBuyerError {
    fn from(e: ProgramError) -> Self {
        match e {
            ProgramError::Custom(code) => match code {
                0 => AutoBuyerError::InvalidInstruction,
                1 => AutoBuyerError::InsufficientFunds,
                2 => AutoBuyerError::InsufficientLiquidity,
                3 => AutoBuyerError::SlippageTooHigh,
                4 => AutoBuyerError::PoolNotFound,
                5 => AutoBuyerError::InvalidParameters,
                6 => AutoBuyerError::InvalidAccountOwner,
                7 => AutoBuyerError::MathOverflow,
                8 => AutoBuyerError::CpiError,
                9 => AutoBuyerError::TokenNotSupported,
                _ => AutoBuyerError::InvalidInstruction,
            },
            _ => AutoBuyerError::InvalidInstruction,
        }
    }
}
