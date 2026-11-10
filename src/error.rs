use pinocchio::program_error::ProgramError;

/// Program error codes
#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AutoBuyerError {
    InvalidInstruction = 0,
    InsufficientFunds = 1,
    InsufficientLiquidity = 2,
    SlippageTooHigh = 3,
    PoolNotFound = 4,
    InvalidParameters = 5,
    InvalidAccountOwner = 6,
    MathOverflow = 7,
    CpiError = 8,
    TokenNotSupported = 9,
}

impl AutoBuyerError {
    /// Convert to program error
    pub fn into_program_error(self) -> ProgramError {
        ProgramError::Custom(self as u32)
    }
}

impl From<AutoBuyerError> for ProgramError {
    fn from(e: AutoBuyerError) -> Self {
        e.into_program_error()
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