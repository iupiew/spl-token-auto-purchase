use crate::error::AutoBuyerError;
use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::program_error::ProgramError; // Removed unused Pubkey import

/// Инструкции для автоматического покупателя токенов
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub enum AutoBuyerInstruction {
    /// Купить токен автоматически
    ///
    /// Аккаунты:
    /// 0. `[signer]` Пользователь
    /// 1. `[writable]` Аккаунт токена-котировки пользователя (источник)
    /// 2. `[writable]` Аккаунт целевого токена пользователя (назначение)
    /// 3. `[]` Минт токена для покупки
    /// 4. `[]` Минт токена-котировки
    /// 5. `[]` Raydium программа
    /// 6. `[writable]` Raydium пул
    /// 7. `[writable]` Raydium пул токен A аккаунт
    /// 8. `[writable]` Raydium пул токен B аккаунт
    /// 9. `[]` Программа токенов
    /// 10. `[]` Системная программа
    BuyToken {
        /// Сумма в токене-котировке для обмена
        amount_in: u64,
        /// Минимальное приемлемое количество выходного токена
        min_amount_out: u64,
    },
}

impl AutoBuyerInstruction {
    /// Упаковать инструкцию в байты
    pub fn pack(&self) -> Vec<u8> {
        borsh::to_vec(self).unwrap()
    }

    /// Распаковать инструкцию из байт
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        borsh::from_slice(input).map_err(|_| AutoBuyerError::InvalidInstruction.into())
    }
}

/// Данные результата покупки
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct BuyResult {
    /// Успешность транзакции
    pub success: bool,
    /// Количество фактически полученного токена
    pub amount_out: u64,
    /// Размер уплаченной комиссии
    pub fee_paid: u64,
    /// Время выполнения транзакции
    pub timestamp: i64,
}
