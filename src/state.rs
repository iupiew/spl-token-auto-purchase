use borsh::{BorshDeserialize, BorshSerialize};
use pinocchio::pubkey::Pubkey;

/// Конфигурация пула ликвидности
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct PoolConfig {
    /// Адрес пула
    pub pool_address: Pubkey,
    /// Адрес аккаунта токена A пула
    pub token_a_account: Pubkey,
    /// Адрес аккаунта токена B пула
    pub token_b_account: Pubkey,
    /// Минт токена A
    pub token_a_mint: Pubkey,
    /// Минт токена B
    pub token_b_mint: Pubkey,
    /// Комиссия пула (в базисных пунктах)
    pub fee_rate: u16,
}

/// Информация о торговой паре
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct TradingPair {
    /// Базовый токен (токен для покупки)
    pub base_mint: Pubkey,
    /// Токен-котировка
    pub quote_mint: Pubkey,
    /// Конфигурация пула
    pub pool_config: PoolConfig,
}

/// Результат расчета обмена
#[derive(Debug, Clone)]
pub struct SwapCalculation {
    /// Количество входного токена
    pub amount_in: u64,
    /// Количество выходного токена
    pub amount_out: u64,
    /// Размер комиссии
    pub fee_amount: u64,
    /// Цена за единицу
    pub price_per_unit: f64,
    /// Проскальзывание в процентах
    pub slippage_percent: f64,
}

/// Константы программы
pub mod constants {
    use pinocchio::pubkey::Pubkey;

    /// WSOL минт адрес
    pub const WSOL_MINT: Pubkey = [
        6, 155, 136, 87, 254, 171, 129, 132, 251, 104, 127, 99, 70, 24, 192, 53, 218, 196, 57, 220,
        26, 235, 59, 85, 152, 160, 240, 0, 0, 0, 0, 1,
    ];

    /// Raydium v4 программа ID
    pub const RAYDIUM_V4_PROGRAM_ID: Pubkey = [
        0x67, 0x5d, 0x14, 0x89, 0xbb, 0x1b, 0x7b, 0x1c, 0x62, 0x5f, 0x9c, 0x3a, 0x5e, 0x2f, 0x8e,
        0x17, 0x8c, 0xa2, 0x7c, 0x8a, 0x5d, 0xf8, 0x2b, 0x36, 0x91, 0x4e, 0x3d, 0x7a, 0x5a, 0x9e,
        0x8b, 0x5c,
    ];

    /// Serum программа ID
    pub const SERUM_PROGRAM_ID: Pubkey = [
        0x9, 0x71, 0x2, 0x4, 0xac, 0x5, 0x39, 0x36, 0x51, 0x2, 0x4, 0xac, 0x5, 0x39, 0x36, 0x51,
        0x2, 0x4, 0xac, 0x5, 0x39, 0x36, 0x51, 0x2, 0x4, 0xac, 0x5, 0x39, 0x36, 0x51, 0x2, 0x4,
    ];

    /// AMM authority
    pub const AMM_AUTHORITY: Pubkey = [
        0x5, 0x1a, 0xda, 0x5b, 0x3b, 0x1d, 0x82, 0x9e, 0x8, 0x2b, 0x57, 0x8, 0x8, 0x4, 0x9, 0x25,
        0x2d, 0x1, 0x4, 0x7, 0x5, 0x4, 0x44, 0x4, 0x4, 0x4, 0x4, 0x4, 0x4, 0x4, 0x4, 0x4,
    ];

    /// Программа токенов SPL
    pub const TOKEN_PROGRAM_ID: Pubkey = [
        6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180, 133,
        237, 95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169,
    ];

    /// Максимальное проскальзывание (1%)
    pub const MAX_SLIPPAGE_BPS: u16 = 100;

    /// Базисные пункты (10000 = 100%)
    pub const BASIS_POINTS: u16 = 10000;
}
