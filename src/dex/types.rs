use crate::state::{SwapCalculation, TradingPair};

/// Supported DEX providers
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DexProvider {
    RaydiumV4,
    // Other DEXes can be added in the future
    // Orca,
    // Serum,
}

/// Parameters for executing a swap
#[derive(Debug, Clone)]
pub struct SwapParams {
    /// Trading pair
    #[allow(dead_code)]
    pub trading_pair: TradingPair,
    /// Input token amount
    pub amount_in: u64,
    /// Minimum output token amount
    pub min_amount_out: u64,
    /// Swap calculation
    pub calculation: SwapCalculation,
}

/// Pool liquidity information
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PoolLiquidity {
    /// Token A reserve
    pub reserve_a: u64,
    /// Token B reserve
    pub reserve_b: u64,
    /// Total LP token supply
    pub total_supply: u64,
}

/// Pool search result
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PoolSearchResult {
    /// Found trading pair
    pub trading_pair: TradingPair,
    /// Liquidity information
    pub liquidity: PoolLiquidity,
    /// Pool score (for selecting best)
    pub score: f64,
}

impl PoolSearchResult {
    /// Calculate pool score based on liquidity and other factors
    #[allow(dead_code)]
    pub fn calculate_score(&mut self) {
        // Simple scoring algorithm: higher liquidity = higher score
        let total_liquidity = self
            .liquidity
            .reserve_a
            .saturating_add(self.liquidity.reserve_b);
        self.score = total_liquidity as f64;
    }
}
