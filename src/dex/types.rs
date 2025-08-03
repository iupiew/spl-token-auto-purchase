use crate::state::{SwapCalculation, TradingPair};

/// Поддерживаемые провайдеры DEX
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DexProvider {
    RaydiumV4,
    // Можно добавить другие DEX в будущем
    // Orca,
    // Serum,
}

/// Параметры для выполнения обмена
#[derive(Debug, Clone)]
pub struct SwapParams {
    /// Торговая пара
    pub trading_pair: TradingPair,
    /// Количество входного токена
    pub amount_in: u64,
    /// Минимальное количество выходного токена
    pub min_amount_out: u64,
    /// Расчет обмена
    pub calculation: SwapCalculation,
}

/// Информация о ликвидности пула
#[derive(Debug, Clone)]
pub struct PoolLiquidity {
    /// Резерв токена A
    pub reserve_a: u64,
    /// Резерв токена B
    pub reserve_b: u64,
    /// Общее количество LP токенов
    pub total_supply: u64,
}

/// Результат поиска пула
#[derive(Debug, Clone)]
pub struct PoolSearchResult {
    /// Найденная торговая пара
    pub trading_pair: TradingPair,
    /// Информация о ликвидности
    pub liquidity: PoolLiquidity,
    /// Рейтинг пула (для выбора лучшего)
    pub score: f64,
}

impl PoolSearchResult {
    /// Рассчитать рейтинг пула на основе ликвидности и других факторов
    pub fn calculate_score(&mut self) {
        // Простой алгоритм оценки: чем больше ликвидность, тем выше рейтинг
        let total_liquidity = self
            .liquidity
            .reserve_a
            .saturating_add(self.liquidity.reserve_b);
        self.score = total_liquidity as f64;
    }
}
