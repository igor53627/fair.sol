//! Deleveraging Cascade Simulation
//!
//! Models death spiral dynamics under different liquidation mechanisms.
//! Key insight: Fair's keeper pool prevents cascades by ensuring liquidations
//! happen even when individual profit incentives are low.
//!
//! ## Cascade Mechanics
//! 1. Price drops -> CDPs become undercollateralized
//! 2. Liquidations sell collateral -> further price impact
//! 3. More CDPs become undercollateralized -> feedback loop
//! 4. If liquidations can't keep up -> bad debt accumulates
//!
//! ## What We Measure
//! - Cascade depth (max sequential liquidation waves)
//! - Bad debt (unliquidated underwater positions)
//! - Time to stability (blocks until no more liquidations)
//! - Price impact (how much liquidations move the price)

use rand::prelude::*;
use rand_distr::{Distribution, Normal};

const NUM_CDPS: usize = 500;
const NUM_KEEPERS: usize = 50;
const INITIAL_ETH_PRICE: f64 = 2000.0;
const LIQUIDATION_PENALTY: f64 = 0.13;
const MIN_COLLATERAL_RATIO: f64 = 1.5; // 150% minimum

const LIQUIDATIONS_PER_BLOCK: usize = 10;
const MAX_BLOCKS: usize = 100;
const PRICE_IMPACT_PER_ETH: f64 = 0.0001; // 0.01% per ETH sold

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LiquidationMechanism {
    Traditional,  // Winner-takes-all, gas priority
    KeeperPool,   // Fair: 70/30 split, commit-reveal
}

impl LiquidationMechanism {
    pub fn all() -> Vec<Self> {
        vec![Self::Traditional, Self::KeeperPool]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Traditional => "Traditional (Winner-Takes-All)",
            Self::KeeperPool => "Fair (Keeper Pool 70/30)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PriceScenario {
    GradualDecline,    // 2% per block for 10 blocks
    FlashCrash,        // 30% instant drop
    VolatileCrash,     // Jump-diffusion with high volatility
    BlackSwan,         // 50% crash + continued decline
}

impl PriceScenario {
    pub fn all() -> Vec<Self> {
        vec![
            Self::GradualDecline,
            Self::FlashCrash,
            Self::VolatileCrash,
            Self::BlackSwan,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::GradualDecline => "Gradual Decline (-20% over 10 blocks)",
            Self::FlashCrash => "Flash Crash (-30% instant)",
            Self::VolatileCrash => "Volatile Crash (jump-diffusion)",
            Self::BlackSwan => "Black Swan (-50% + continued decline)",
        }
    }
}

#[derive(Clone)]
struct CDP {
    id: usize,
    collateral: f64,      // ETH
    debt: f64,            // USD
    is_liquidated: bool,
}

impl CDP {
    fn new(id: usize, rng: &mut impl Rng) -> Self {
        let collateral = 1.0 + rng.gen::<f64>() * 19.0; // 1-20 ETH
        let ratio = 1.5 + rng.gen::<f64>() * 1.0; // 150-250% initial ratio
        let debt = (collateral * INITIAL_ETH_PRICE) / ratio;
        
        Self {
            id,
            collateral,
            debt,
            is_liquidated: false,
        }
    }

    fn collateral_ratio(&self, eth_price: f64) -> f64 {
        if self.debt == 0.0 {
            return f64::INFINITY;
        }
        (self.collateral * eth_price) / self.debt
    }

    fn is_underwater(&self, eth_price: f64) -> bool {
        self.collateral_ratio(eth_price) < 1.0
    }

    fn is_liquidatable(&self, eth_price: f64) -> bool {
        !self.is_liquidated && self.collateral_ratio(eth_price) < MIN_COLLATERAL_RATIO
    }

    fn liquidation_profit(&self, eth_price: f64) -> f64 {
        let collateral_value = self.collateral * eth_price;
        let profit = (collateral_value - self.debt) * LIQUIDATION_PENALTY;
        profit.max(0.0)
    }

    fn bad_debt(&self, eth_price: f64) -> f64 {
        if self.is_underwater(eth_price) && !self.is_liquidated {
            (self.debt - self.collateral * eth_price).max(0.0)
        } else {
            0.0
        }
    }
}

#[derive(Clone)]
struct Keeper {
    id: usize,
    capital: f64,         // Available capital for liquidations
    gas_priority: f64,    // 0-1, higher = faster execution
    total_profit: f64,
    liquidations: usize,
}

impl Keeper {
    fn new(id: usize, rng: &mut impl Rng) -> Self {
        Self {
            id,
            capital: 10000.0 + rng.gen::<f64>() * 90000.0, // $10k-$100k
            gas_priority: rng.gen::<f64>(),
            total_profit: 0.0,
            liquidations: 0,
        }
    }

    fn willing_to_liquidate(&self, profit: f64, mechanism: LiquidationMechanism) -> bool {
        match mechanism {
            LiquidationMechanism::Traditional => {
                profit > 50.0 // Only if profit > gas cost
            }
            LiquidationMechanism::KeeperPool => {
                profit > 10.0 // Lower threshold because of shared profit
            }
        }
    }
}

struct CascadeSimulation {
    cdps: Vec<CDP>,
    keepers: Vec<Keeper>,
    eth_price: f64,
    mechanism: LiquidationMechanism,
    scenario: PriceScenario,
    
    block: usize,
    cascade_depth: usize,
    current_wave_liquidations: usize,
    total_liquidations: usize,
    total_bad_debt: f64,
    price_history: Vec<f64>,
    liquidations_per_block: Vec<usize>,
}

impl CascadeSimulation {
    fn new(mechanism: LiquidationMechanism, scenario: PriceScenario, rng: &mut impl Rng) -> Self {
        let cdps: Vec<CDP> = (0..NUM_CDPS).map(|i| CDP::new(i, rng)).collect();
        let keepers: Vec<Keeper> = (0..NUM_KEEPERS).map(|i| Keeper::new(i, rng)).collect();
        
        Self {
            cdps,
            keepers,
            eth_price: INITIAL_ETH_PRICE,
            mechanism,
            scenario,
            block: 0,
            cascade_depth: 0,
            current_wave_liquidations: 0,
            total_liquidations: 0,
            total_bad_debt: 0.0,
            price_history: vec![INITIAL_ETH_PRICE],
            liquidations_per_block: Vec::new(),
        }
    }

    fn apply_price_shock(&mut self, rng: &mut impl Rng) {
        match self.scenario {
            PriceScenario::GradualDecline => {
                if self.block < 10 {
                    self.eth_price *= 0.98; // 2% drop per block
                }
            }
            PriceScenario::FlashCrash => {
                if self.block == 0 {
                    self.eth_price *= 0.70; // 30% instant drop
                }
            }
            PriceScenario::VolatileCrash => {
                let normal = Normal::new(-0.02, 0.05).unwrap();
                let return_pct: f64 = normal.sample(rng);
                self.eth_price *= 1.0 + return_pct;
                
                if rng.gen::<f64>() < 0.1 {
                    self.eth_price *= 0.9; // 10% chance of 10% jump down
                }
            }
            PriceScenario::BlackSwan => {
                if self.block == 0 {
                    self.eth_price *= 0.50; // 50% instant drop
                } else if self.block < 20 {
                    self.eth_price *= 0.99; // Continued 1% decline
                }
            }
        }
        
        self.eth_price = self.eth_price.max(100.0);
        self.price_history.push(self.eth_price);
    }

    fn apply_liquidation_price_impact(&mut self, eth_sold: f64) {
        let impact = eth_sold * PRICE_IMPACT_PER_ETH;
        self.eth_price *= 1.0 - impact;
        self.eth_price = self.eth_price.max(100.0);
    }

    fn run_liquidation_round(&mut self, rng: &mut impl Rng) -> usize {
        let mut liquidatable: Vec<usize> = self.cdps.iter()
            .enumerate()
            .filter(|(_, cdp)| cdp.is_liquidatable(self.eth_price))
            .map(|(i, _)| i)
            .collect();
        
        liquidatable.sort_by(|&a, &b| {
            let ratio_a = self.cdps[a].collateral_ratio(self.eth_price);
            let ratio_b = self.cdps[b].collateral_ratio(self.eth_price);
            ratio_a.partial_cmp(&ratio_b).unwrap()
        });
        
        let mut liquidations_this_block = 0;
        let mut eth_sold_this_block = 0.0;
        
        for cdp_idx in liquidatable.iter().take(LIQUIDATIONS_PER_BLOCK) {
            let cdp = &self.cdps[*cdp_idx];
            let profit = cdp.liquidation_profit(self.eth_price);
            
            let participating_keepers: Vec<usize> = self.keepers.iter()
                .enumerate()
                .filter(|(_, k)| k.willing_to_liquidate(profit, self.mechanism))
                .map(|(i, _)| i)
                .collect();
            
            if participating_keepers.is_empty() {
                continue;
            }
            
            match self.mechanism {
                LiquidationMechanism::Traditional => {
                    let winner_idx = participating_keepers.iter()
                        .max_by(|&&a, &&b| {
                            self.keepers[a].gas_priority
                                .partial_cmp(&self.keepers[b].gas_priority)
                                .unwrap()
                        })
                        .unwrap();
                    
                    self.keepers[*winner_idx].total_profit += profit;
                    self.keepers[*winner_idx].liquidations += 1;
                }
                LiquidationMechanism::KeeperPool => {
                    let keeper_share = profit * 0.7;
                    let per_keeper = keeper_share / participating_keepers.len() as f64;
                    
                    for &k_idx in &participating_keepers {
                        self.keepers[k_idx].total_profit += per_keeper;
                    }
                    
                    let winner_idx = participating_keepers[rng.gen_range(0..participating_keepers.len())];
                    self.keepers[winner_idx].liquidations += 1;
                }
            }
            
            eth_sold_this_block += self.cdps[*cdp_idx].collateral;
            self.cdps[*cdp_idx].is_liquidated = true;
            liquidations_this_block += 1;
        }
        
        self.apply_liquidation_price_impact(eth_sold_this_block);
        
        liquidations_this_block
    }

    fn calculate_bad_debt(&self) -> f64 {
        self.cdps.iter()
            .map(|cdp| cdp.bad_debt(self.eth_price))
            .sum()
    }

    fn run(&mut self, rng: &mut impl Rng) -> CascadeResult {
        let mut consecutive_empty_blocks = 0;
        let mut max_wave_liquidations = 0;
        
        while self.block < MAX_BLOCKS {
            self.apply_price_shock(rng);
            
            let liquidations = self.run_liquidation_round(rng);
            self.liquidations_per_block.push(liquidations);
            self.total_liquidations += liquidations;
            
            if liquidations > 0 {
                self.current_wave_liquidations += liquidations;
                max_wave_liquidations = max_wave_liquidations.max(liquidations);
                consecutive_empty_blocks = 0;
            } else {
                if self.current_wave_liquidations > 0 {
                    self.cascade_depth += 1;
                }
                self.current_wave_liquidations = 0;
                consecutive_empty_blocks += 1;
                
                if consecutive_empty_blocks >= 5 && self.block > 10 {
                    break;
                }
            }
            
            self.block += 1;
        }
        
        self.total_bad_debt = self.calculate_bad_debt();
        
        let keeper_profits: Vec<f64> = self.keepers.iter().map(|k| k.total_profit).collect();
        let total_profit: f64 = keeper_profits.iter().sum();
        
        let profit_concentration = if total_profit > 0.0 {
            let mut sorted_profits = keeper_profits.clone();
            sorted_profits.sort_by(|a, b| b.partial_cmp(a).unwrap());
            let top_20_pct: f64 = sorted_profits.iter().take(NUM_KEEPERS / 5).sum();
            top_20_pct / total_profit
        } else {
            0.0
        };
        
        let participation_rate = self.keepers.iter()
            .filter(|k| k.liquidations > 0)
            .count() as f64 / NUM_KEEPERS as f64;
        
        let price_drop = 1.0 - (self.eth_price / INITIAL_ETH_PRICE);
        
        let unliquidated_underwater: usize = self.cdps.iter()
            .filter(|cdp| cdp.is_underwater(self.eth_price) && !cdp.is_liquidated)
            .count();
        
        CascadeResult {
            mechanism: self.mechanism,
            scenario: self.scenario,
            cascade_depth: self.cascade_depth,
            total_liquidations: self.total_liquidations,
            bad_debt: self.total_bad_debt,
            blocks_to_stability: self.block,
            final_price: self.eth_price,
            price_drop_pct: price_drop * 100.0,
            profit_concentration,
            participation_rate,
            unliquidated_underwater,
            max_liquidations_per_block: *self.liquidations_per_block.iter().max().unwrap_or(&0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CascadeResult {
    pub mechanism: LiquidationMechanism,
    pub scenario: PriceScenario,
    pub cascade_depth: usize,
    pub total_liquidations: usize,
    pub bad_debt: f64,
    pub blocks_to_stability: usize,
    pub final_price: f64,
    pub price_drop_pct: f64,
    pub profit_concentration: f64,
    pub participation_rate: f64,
    pub unliquidated_underwater: usize,
    pub max_liquidations_per_block: usize,
}

pub fn run_cascade_simulation(
    mechanism: LiquidationMechanism,
    scenario: PriceScenario,
    runs: usize,
) -> Vec<CascadeResult> {
    let mut rng = rand::thread_rng();
    
    (0..runs)
        .map(|_| {
            let mut sim = CascadeSimulation::new(mechanism, scenario, &mut rng);
            sim.run(&mut rng)
        })
        .collect()
}

pub fn aggregate_results(results: &[CascadeResult]) -> AggregatedCascadeResult {
    let n = results.len() as f64;
    
    AggregatedCascadeResult {
        mechanism: results[0].mechanism,
        scenario: results[0].scenario,
        runs: results.len(),
        avg_cascade_depth: results.iter().map(|r| r.cascade_depth as f64).sum::<f64>() / n,
        avg_liquidations: results.iter().map(|r| r.total_liquidations as f64).sum::<f64>() / n,
        avg_bad_debt: results.iter().map(|r| r.bad_debt).sum::<f64>() / n,
        max_bad_debt: results.iter().map(|r| r.bad_debt).fold(0.0, f64::max),
        avg_blocks_to_stability: results.iter().map(|r| r.blocks_to_stability as f64).sum::<f64>() / n,
        avg_price_drop_pct: results.iter().map(|r| r.price_drop_pct).sum::<f64>() / n,
        avg_profit_concentration: results.iter().map(|r| r.profit_concentration).sum::<f64>() / n,
        avg_participation_rate: results.iter().map(|r| r.participation_rate).sum::<f64>() / n,
        avg_unliquidated: results.iter().map(|r| r.unliquidated_underwater as f64).sum::<f64>() / n,
        bad_debt_frequency: results.iter().filter(|r| r.bad_debt > 0.0).count() as f64 / n,
    }
}

#[derive(Debug)]
pub struct AggregatedCascadeResult {
    pub mechanism: LiquidationMechanism,
    pub scenario: PriceScenario,
    pub runs: usize,
    pub avg_cascade_depth: f64,
    pub avg_liquidations: f64,
    pub avg_bad_debt: f64,
    pub max_bad_debt: f64,
    pub avg_blocks_to_stability: f64,
    pub avg_price_drop_pct: f64,
    pub avg_profit_concentration: f64,
    pub avg_participation_rate: f64,
    pub avg_unliquidated: f64,
    pub bad_debt_frequency: f64,
}

impl AggregatedCascadeResult {
    pub fn print(&self) {
        println!("  Avg cascade depth:       {:.1} waves", self.avg_cascade_depth);
        println!("  Avg liquidations:        {:.1}", self.avg_liquidations);
        println!("  Avg bad debt:            ${:.0}", self.avg_bad_debt);
        println!("  Max bad debt:            ${:.0}", self.max_bad_debt);
        println!("  Bad debt frequency:      {:.1}%", self.bad_debt_frequency * 100.0);
        println!("  Avg blocks to stable:    {:.1}", self.avg_blocks_to_stability);
        println!("  Avg price drop:          {:.1}%", self.avg_price_drop_pct);
        println!("  Profit concentration:    {:.1}%", self.avg_profit_concentration * 100.0);
        println!("  Keeper participation:    {:.1}%", self.avg_participation_rate * 100.0);
        println!("  Avg unliquidated:        {:.1} CDPs", self.avg_unliquidated);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cdp_collateral_ratio() {
        let cdp = CDP {
            id: 0,
            collateral: 10.0,
            debt: 10000.0,
            is_liquidated: false,
        };
        
        assert!((cdp.collateral_ratio(2000.0) - 2.0).abs() < 0.001);
        assert!((cdp.collateral_ratio(1000.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cascade_simulation_runs() {
        let results = run_cascade_simulation(
            LiquidationMechanism::KeeperPool,
            PriceScenario::FlashCrash,
            10,
        );
        
        assert_eq!(results.len(), 10);
        for r in &results {
            assert!(r.total_liquidations > 0);
        }
    }

    #[test]
    fn test_keeper_pool_better_participation() {
        let traditional = run_cascade_simulation(
            LiquidationMechanism::Traditional,
            PriceScenario::FlashCrash,
            100,
        );
        let keeper_pool = run_cascade_simulation(
            LiquidationMechanism::KeeperPool,
            PriceScenario::FlashCrash,
            100,
        );
        
        let trad_agg = aggregate_results(&traditional);
        let pool_agg = aggregate_results(&keeper_pool);
        
        println!("Traditional participation: {:.1}%", trad_agg.avg_participation_rate * 100.0);
        println!("Keeper Pool participation: {:.1}%", pool_agg.avg_participation_rate * 100.0);
        
        assert!(pool_agg.avg_participation_rate >= trad_agg.avg_participation_rate);
    }
}
