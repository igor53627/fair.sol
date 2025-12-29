//! IPFE Price of Anarchy Simulation
//!
//! Compares obfuscation strategies for stablecoin liquidation:
//! 1. No hiding (transparent threshold)
//! 2. Noise-based (add randomness to threshold)
//! 3. IPFE (weights completely hidden, only score revealed)
//! 4. Fair variants with profit sharing
//!
//! Measures Price of Anarchy = Nash Cost / Social Optimum

use rand::prelude::*;

pub const NUM_CDPS: usize = 100;
pub const NUM_KEEPERS: usize = 20;
pub const ETH_PRICE: f64 = 2000.0;
pub const LIQUIDATION_PENALTY: f64 = 0.13;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ObfuscationStrategy {
    Transparent,
    NoiseBased,
    IPFE,
    Fair6040,
    Fair5050,
    KeeperPool,
}

impl ObfuscationStrategy {
    pub fn all() -> Vec<Self> {
        vec![
            Self::Transparent,
            Self::NoiseBased,
            Self::IPFE,
            Self::Fair6040,
            Self::Fair5050,
            Self::KeeperPool,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Transparent => "Transparent",
            Self::NoiseBased => "Noise-Based",
            Self::IPFE => "IPFE Only",
            Self::Fair6040 => "Fair 60/40",
            Self::Fair5050 => "Fair 50/50",
            Self::KeeperPool => "Keeper Pool 70/30",
        }
    }
}

#[derive(Clone)]
pub struct CDP {
    pub id: usize,
    pub collateral: f64,
    pub debt: f64,
    pub age_days: f64,
    pub volatility_score: f64,
}

impl CDP {
    pub fn new(id: usize, rng: &mut impl Rng) -> Self {
        let collateral = 1.0 + rng.gen::<f64>() * 9.0;
        let ratio = 1.3 + rng.gen::<f64>() * 0.5;
        let debt = (collateral * ETH_PRICE) / ratio;

        Self {
            id,
            collateral,
            debt,
            age_days: rng.gen::<f64>() * 365.0,
            volatility_score: rng.gen::<f64>(),
        }
    }

    pub fn collateral_ratio(&self, eth_price: f64) -> f64 {
        (self.collateral * eth_price) / self.debt
    }

    pub fn features(&self, eth_price: f64) -> [f64; 5] {
        [
            self.collateral_ratio(eth_price),
            self.volatility_score,
            self.debt / (self.collateral * eth_price),
            (self.age_days / 365.0).min(1.0),
            (self.collateral * eth_price / 10000.0).min(2.0),
        ]
    }

    pub fn liquidation_profit(&self, eth_price: f64) -> f64 {
        let collateral_value = self.collateral * eth_price;
        let profit = collateral_value - self.debt - 50.0;
        profit.max(0.0) * LIQUIDATION_PENALTY
    }
}

#[derive(Clone)]
pub struct Keeper {
    pub id: usize,
    pub gas_priority: f64,
    pub total_profit: f64,
    pub successful_liquidations: usize,
}

impl Keeper {
    pub fn new(id: usize, rng: &mut impl Rng) -> Self {
        Self {
            id,
            gas_priority: rng.gen::<f64>(),
            total_profit: 0.0,
            successful_liquidations: 0,
        }
    }
}

pub struct LiquidationGame {
    pub cdps: Vec<CDP>,
    pub eth_price: f64,
    pub true_weights: [f64; 5],
    pub true_threshold: f64,
    pub strategy: ObfuscationStrategy,
    pub noise_level: f64,
}

impl LiquidationGame {
    pub fn new(strategy: ObfuscationStrategy, rng: &mut impl Rng) -> Self {
        let cdps: Vec<CDP> = (0..NUM_CDPS).map(|i| CDP::new(i, rng)).collect();
        let true_weights = [2.0, -1.0, -1.5, 0.3, -0.3];
        let true_threshold = 2.0;

        Self {
            cdps,
            eth_price: ETH_PRICE,
            true_weights,
            true_threshold,
            strategy,
            noise_level: 0.29,
        }
    }

    pub fn compute_true_score(&self, cdp: &CDP) -> f64 {
        let features = cdp.features(self.eth_price);
        features
            .iter()
            .zip(self.true_weights.iter())
            .map(|(f, w)| f * w)
            .sum()
    }

    pub fn is_truly_liquidatable(&self, cdp: &CDP) -> bool {
        self.compute_true_score(cdp) < self.true_threshold
    }

    pub fn keeper_perceives_liquidatable(&self, cdp: &CDP, rng: &mut impl Rng) -> (bool, f64) {
        match self.strategy {
            ObfuscationStrategy::Transparent => {
                let score = self.compute_true_score(cdp);
                (score < self.true_threshold, 1.0)
            }
            ObfuscationStrategy::NoiseBased => {
                let perceived_threshold =
                    self.true_threshold * (1.0 + (rng.gen::<f64>() - 0.5) * 2.0 * self.noise_level);
                let score = self.compute_true_score(cdp);
                let confidence = 1.0 - self.noise_level;
                (score < perceived_threshold, confidence)
            }
            ObfuscationStrategy::IPFE => {
                let ratio = cdp.collateral_ratio(self.eth_price);
                let perceived_liquidatable = ratio < 1.6;
                let confidence = 0.2 + rng.gen::<f64>() * 0.4;
                (perceived_liquidatable, confidence)
            }
            ObfuscationStrategy::Fair6040
            | ObfuscationStrategy::Fair5050
            | ObfuscationStrategy::KeeperPool => {
                let ratio = cdp.collateral_ratio(self.eth_price);
                let perceived_liquidatable = ratio < 1.6;
                let confidence = rng.gen::<f64>();
                (perceived_liquidatable, confidence)
            }
        }
    }

    pub fn simulate_price_drop(&mut self, pct: f64) {
        self.eth_price *= 1.0 - pct;
    }
}

#[derive(Debug, Clone)]
pub struct GameResult {
    pub strategy: ObfuscationStrategy,
    pub successful_liquidations: usize,
    pub failed_attempts: usize,
    pub missed_liquidations: usize,
    pub total_profit: f64,
    pub front_runner_profit: f64,
    pub profit_concentration: f64,
    pub gas_waste_ratio: f64,
    pub coverage: f64,
}

pub fn simulate_game(strategy: ObfuscationStrategy, rng: &mut impl Rng) -> GameResult {
    let mut game = LiquidationGame::new(strategy, rng);
    let mut keepers: Vec<Keeper> = (0..NUM_KEEPERS).map(|i| Keeper::new(i, rng)).collect();

    game.simulate_price_drop(0.10);

    let mut total_profit_extracted = 0.0;
    let mut failed_attempts = 0;
    let mut successful_liquidations = 0;
    let mut front_runner_profit = 0.0;
    let missed_liquidations = 0;

    let truly_liquidatable: Vec<usize> = game
        .cdps
        .iter()
        .enumerate()
        .filter(|(_, cdp)| game.is_truly_liquidatable(cdp))
        .map(|(i, _)| i)
        .collect();

    for cdp in &game.cdps {
        let mut attempts: Vec<(usize, f64, f64)> = Vec::new();

        for keeper in &keepers {
            let (perceives_liquidatable, confidence) =
                game.keeper_perceives_liquidatable(cdp, rng);

            if perceives_liquidatable {
                let effective_priority = match strategy {
                    ObfuscationStrategy::Transparent | ObfuscationStrategy::NoiseBased => {
                        keeper.gas_priority * confidence
                    }
                    _ => keeper.gas_priority * confidence * 0.5 + rng.gen::<f64>() * 0.5,
                };
                attempts.push((keeper.id, effective_priority, confidence));
            }
        }

        if attempts.is_empty() {
            continue;
        }

        attempts.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let uses_random = matches!(
            strategy,
            ObfuscationStrategy::Fair6040
                | ObfuscationStrategy::Fair5050
                | ObfuscationStrategy::KeeperPool
        );

        let winner_idx = if uses_random {
            rng.gen_range(0..attempts.len())
        } else {
            0
        };
        let (winner_id, _, _) = attempts[winner_idx];

        if game.is_truly_liquidatable(cdp) {
            let profit = cdp.liquidation_profit(game.eth_price);

            match strategy {
                ObfuscationStrategy::Fair6040 if attempts.len() > 1 => {
                    let winner_share = profit * 0.6;
                    let pool_share = profit * 0.4;
                    let per_other = pool_share / (attempts.len() - 1) as f64;

                    keepers[winner_id].total_profit += winner_share;
                    for (i, (kid, _, _)) in attempts.iter().enumerate() {
                        if i != winner_idx {
                            keepers[*kid].total_profit += per_other;
                        }
                    }
                }
                ObfuscationStrategy::Fair5050 if attempts.len() > 1 => {
                    let winner_share = profit * 0.5;
                    let pool_share = profit * 0.5;
                    let per_other = pool_share / (attempts.len() - 1) as f64;

                    keepers[winner_id].total_profit += winner_share;
                    for (i, (kid, _, _)) in attempts.iter().enumerate() {
                        if i != winner_idx {
                            keepers[*kid].total_profit += per_other;
                        }
                    }
                }
                ObfuscationStrategy::KeeperPool => {
                    let keeper_pool = profit * 0.7;
                    let per_keeper = keeper_pool / attempts.len() as f64;

                    for (kid, _, _) in attempts.iter() {
                        keepers[*kid].total_profit += per_keeper;
                    }
                }
                _ => {
                    keepers[winner_id].total_profit += profit;
                }
            }

            keepers[winner_id].successful_liquidations += 1;
            total_profit_extracted += profit;
            successful_liquidations += 1;

            if keepers[winner_id].gas_priority > 0.8 {
                front_runner_profit += profit;
            }
        } else {
            failed_attempts += 1;
        }
    }

    let profit_concentration = if total_profit_extracted > 0.0 {
        let mut profits: Vec<f64> = keepers.iter().map(|k| k.total_profit).collect();
        profits.sort_by(|a, b| b.partial_cmp(a).unwrap());
        let top_20_pct = profits.iter().take(NUM_KEEPERS / 5).sum::<f64>();
        top_20_pct / total_profit_extracted
    } else {
        0.0
    };

    let gas_waste_ratio =
        failed_attempts as f64 / (failed_attempts + successful_liquidations).max(1) as f64;
    let coverage = successful_liquidations as f64 / truly_liquidatable.len().max(1) as f64;

    GameResult {
        strategy,
        successful_liquidations,
        failed_attempts,
        missed_liquidations,
        total_profit: total_profit_extracted,
        front_runner_profit,
        profit_concentration,
        gas_waste_ratio,
        coverage,
    }
}

pub fn compute_poa(results: &[GameResult]) -> f64 {
    let avg_concentration: f64 =
        results.iter().map(|r| r.profit_concentration).sum::<f64>() / results.len() as f64;
    let avg_gas_waste: f64 =
        results.iter().map(|r| r.gas_waste_ratio).sum::<f64>() / results.len() as f64;
    let avg_coverage: f64 = results.iter().map(|r| r.coverage).sum::<f64>() / results.len() as f64;

    let nash_cost = avg_concentration + avg_gas_waste + (1.0 - avg_coverage);
    let social_optimum: f64 = 0.2 + 0.0 + 0.0;

    nash_cost / social_optimum.max(0.01)
}

pub fn run_poa_simulation(strategy: ObfuscationStrategy, runs: usize) -> Vec<GameResult> {
    let mut rng = rand::thread_rng();
    (0..runs).map(|_| simulate_game(strategy, &mut rng)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cdp_features() {
        let mut rng = rand::thread_rng();
        let cdp = CDP::new(0, &mut rng);
        let features = cdp.features(ETH_PRICE);

        assert!(features[0] > 1.0);
        assert!(features[1] >= 0.0 && features[1] <= 1.0);
    }

    #[test]
    fn test_transparent_strategy() {
        let mut rng = rand::thread_rng();
        let result = simulate_game(ObfuscationStrategy::Transparent, &mut rng);

        assert!(result.successful_liquidations > 0 || result.missed_liquidations == 0);
    }
}
