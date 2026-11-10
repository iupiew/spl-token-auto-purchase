use pinocchio::pubkey::Pubkey;

/// Pool configuration
#[derive(Debug, Clone, Copy)]
pub struct PoolConfig {
    /// Pool address
    pub pool_address: Pubkey,
    /// Pool token A vault
    pub token_a_account: Pubkey,
    /// Pool token B vault
    pub token_b_account: Pubkey,
    /// Token A mint
    pub token_a_mint: Pubkey,
    /// Token B mint
    pub token_b_mint: Pubkey,
    /// Pool fee rate in basis points
    pub fee_rate: u16,
}

/// Trading pair information
#[derive(Debug, Clone, Copy)]
pub struct TradingPair {
    /// Base token (token to buy)
    pub base_mint: Pubkey,
    /// Quote token
    pub quote_mint: Pubkey,
    /// Pool configuration
    pub pool_config: PoolConfig,
}

/// Swap calculation result
#[derive(Debug, Clone, Copy)]
pub struct SwapCalculation {
    /// Input token amount
    pub amount_in: u64,
    /// Output token amount
    pub amount_out: u64,
    /// Fee amount
    pub fee_amount: u64,
    /// Price per unit (fixed point, scaled by 1e9)
    pub price_per_unit: u64,
    /// Slippage in basis points
    pub slippage_bps: u16,
}

/// Program constants
pub mod constants {
    use pinocchio::pubkey::Pubkey;

    /// WSOL mint address
    pub const WSOL_MINT: Pubkey = [
        6, 155, 136, 87, 254, 171, 129, 132, 251, 104, 127, 99, 70, 24, 192, 53, 218, 196, 57, 220,
        26, 235, 59, 85, 152, 160, 240, 0, 0, 0, 0, 1,
    ];

    /// Raydium v4 program ID (mainnet: 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8)
    pub const RAYDIUM_V4_PROGRAM_ID: Pubkey = [
        75, 217, 73, 196, 54, 2, 195, 63, 32, 119, 144, 237, 22, 163, 82, 76, 161, 185, 151, 92,
        241, 33, 162, 169, 12, 255, 236, 125, 248, 182, 138, 205,
    ];

    /// SPL Token program ID
    pub const TOKEN_PROGRAM_ID: Pubkey = [
        6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180, 133,
        237, 95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169,
    ];

    /// Maximum slippage (1% = 100 bps)
    pub const MAX_SLIPPAGE_BPS: u16 = 100;

    /// Basis points (10000 = 100%)
    pub const BASIS_POINTS: u16 = 10000;

    /// Raydium AMM authority PDA seed
    pub const AMM_AUTHORITY_SEED: &[u8] = b"amm authority";
}

#[cfg(test)]
mod tests {
    use super::constants::*;

    #[test]
    fn test_raydium_v4_program_id_matches_base58() {
        // 675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8
        let expected = [
            75, 217, 73, 196, 54, 2, 195, 63, 32, 119, 144, 237, 22, 163, 82, 76, 161, 185, 151, 92,
            241, 33, 162, 169, 12, 255, 236, 125, 248, 182, 138, 205,
        ];
        assert_eq!(RAYDIUM_V4_PROGRAM_ID, expected);
    }

    #[test]
    fn test_wsol_mint_matches_base58() {
        // So11111111111111111111111111111111111111112
        let expected = [
            6, 155, 136, 87, 254, 171, 129, 132, 251, 104, 127, 99, 70, 24, 192, 53, 218, 196, 57, 220,
            26, 235, 59, 85, 152, 160, 240, 0, 0, 0, 0, 1,
        ];
        assert_eq!(WSOL_MINT, expected);
    }

    #[test]
    fn test_token_program_id_matches_base58() {
        // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let expected = [
            6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180, 133,
            237, 95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169,
        ];
        assert_eq!(TOKEN_PROGRAM_ID, expected);
    }
}
