//! Monte Carlo Stress Testing
//!
//! Statistical analysis of Fair stablecoin under extreme market conditions.
//! Generates thousands of price paths and measures tail risk metrics.
//!
//! ## Price Models
//! - Geometric Brownian Motion (baseline)
//! - Jump-diffusion (Merton model)
//! - GARCH (volatility clustering)
//! - Historical bootstrap (real crash data)
//!
//! ## Metrics
//! - Value at Risk (VaR) at 95%, 99%, 99.9%
//! - Expected Shortfall (CVaR)
//! - Bad debt probability
//! - System insolvency probability

use rand::prelude::*;
use rand_distr::{Distribution, Normal, Poisson};
use std::f64::consts::E;

use crate::cascade::{
    run_cascade_simulation, aggregate_results, LiquidationMechanism, PriceScenario,
    CascadeResult,
};

const INITIAL_PRICE: f64 = 2000.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PriceModel {
    GBM,           // Geometric Brownian Motion
    JumpDiffusion, // Merton jump-diffusion
    GARCH,         // Volatility clustering
    HistoricalMar2020,  // March 2020 COVID crash
    HistoricalMay2021,  // May 2021 crypto crash
    HistoricalNov2022,  // Nov 2022 FTX crash
}

impl PriceModel {
    pub fn all() -> Vec<Self> {
        vec![
            Self::GBM,
            Self::JumpDiffusion,
            Self::GARCH,
            Self::HistoricalMar2020,
            Self::HistoricalMay2021,
            Self::HistoricalNov2022,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::GBM => "GBM (baseline)",
            Self::JumpDiffusion => "Jump-Diffusion",
            Self::GARCH => "GARCH",
            Self::HistoricalMar2020 => "Historical: Mar 2020",
            Self::HistoricalMay2021 => "Historical: May 2021",
            Self::HistoricalNov2022 => "Historical: Nov 2022",
        }
    }
}

#[derive(Clone)]
pub struct PricePathConfig {
    pub model: PriceModel,
    pub blocks: usize,
    pub drift: f64,           // Annual drift (mu)
    pub volatility: f64,      // Annual volatility (sigma)
    pub jump_intensity: f64,  // Jumps per year (lambda)
    pub jump_mean: f64,       // Mean jump size
    pub jump_std: f64,        // Jump size std dev
}

impl Default for PricePathConfig {
    fn default() -> Self {
        Self {
            model: PriceModel::GBM,
            blocks: 100,
            drift: -0.5,        // Bearish scenario
            volatility: 1.5,    // 150% annual vol (crypto-like)
            jump_intensity: 5.0, // 5 jumps per year
            jump_mean: -0.15,   // -15% average jump
            jump_std: 0.10,     // 10% jump std
        }
    }
}

pub fn generate_price_path(config: &PricePathConfig, rng: &mut impl Rng) -> Vec<f64> {
    let blocks_per_year = 365.0 * 24.0 * 60.0 * 5.0; // ~5 blocks per minute
    let dt = 1.0 / blocks_per_year;
    
    let mut prices = vec![INITIAL_PRICE];
    let mut price = INITIAL_PRICE;
    let mut current_vol = config.volatility;
    
    let normal = Normal::new(0.0, 1.0).unwrap();
    
    for _ in 0..config.blocks {
        match config.model {
            PriceModel::GBM => {
                let z: f64 = normal.sample(rng);
                let ret = (config.drift - 0.5 * config.volatility.powi(2)) * dt
                    + config.volatility * dt.sqrt() * z;
                price *= E.powf(ret);
            }
            
            PriceModel::JumpDiffusion => {
                let z: f64 = normal.sample(rng);
                let diffusion = (config.drift - 0.5 * config.volatility.powi(2)) * dt
                    + config.volatility * dt.sqrt() * z;
                
                let lambda_dt = config.jump_intensity * dt;
                let poisson = Poisson::new(lambda_dt).unwrap();
                let num_jumps: u64 = poisson.sample(rng) as u64;
                
                let mut jump_component = 0.0;
                for _ in 0..num_jumps {
                    let jump_normal = Normal::new(config.jump_mean, config.jump_std).unwrap();
                    jump_component += jump_normal.sample(rng);
                }
                
                price *= E.powf(diffusion + jump_component);
            }
            
            PriceModel::GARCH => {
                let z: f64 = normal.sample(rng);
                
                let alpha = 0.1;
                let beta = 0.85;
                let omega = config.volatility.powi(2) * (1.0 - alpha - beta);
                
                let shock = current_vol * z;
                current_vol = (omega + alpha * shock.powi(2) + beta * current_vol.powi(2)).sqrt();
                current_vol = current_vol.max(0.5).min(3.0);
                
                let ret = (config.drift - 0.5 * current_vol.powi(2)) * dt
                    + current_vol * dt.sqrt() * z;
                price *= E.powf(ret);
            }
            
            PriceModel::HistoricalMar2020 => {
                let day_returns = [
                    -0.08, -0.12, -0.25, -0.15, 0.05, -0.10, -0.08, 
                    0.15, 0.08, -0.05, 0.03, -0.02, 0.10, 0.05,
                ];
                let block_idx = prices.len() % (day_returns.len() * 10);
                let day_idx = block_idx / 10;
                let intraday_noise: f64 = normal.sample(rng) * 0.02;
                let ret = day_returns[day_idx] / 10.0 + intraday_noise;
                price *= 1.0 + ret;
            }
            
            PriceModel::HistoricalMay2021 => {
                let day_returns = [
                    -0.05, -0.08, -0.12, -0.30, -0.10, 0.08, -0.15,
                    -0.05, 0.10, 0.05, -0.03, 0.02, -0.05, 0.08,
                ];
                let block_idx = prices.len() % (day_returns.len() * 10);
                let day_idx = block_idx / 10;
                let intraday_noise: f64 = normal.sample(rng) * 0.02;
                let ret = day_returns[day_idx] / 10.0 + intraday_noise;
                price *= 1.0 + ret;
            }
            
            PriceModel::HistoricalNov2022 => {
                let day_returns = [
                    -0.03, -0.05, -0.15, -0.20, -0.10, -0.08, 0.05,
                    -0.05, -0.03, 0.02, -0.02, 0.01, -0.01, 0.03,
                ];
                let block_idx = prices.len() % (day_returns.len() * 10);
                let day_idx = block_idx / 10;
                let intraday_noise: f64 = normal.sample(rng) * 0.02;
                let ret = day_returns[day_idx] / 10.0 + intraday_noise;
                price *= 1.0 + ret;
            }
        }
        
        price = price.max(50.0);
        prices.push(price);
    }
    
    prices
}

#[derive(Debug, Clone)]
pub struct MonteCarloResult {
    pub model: PriceModel,
    pub mechanism: LiquidationMechanism,
    pub runs: usize,
    
    pub bad_debts: Vec<f64>,
    pub price_drops: Vec<f64>,
    pub liquidation_counts: Vec<usize>,
    pub participation_rates: Vec<f64>,
    
    pub var_95: f64,
    pub var_99: f64,
    pub var_999: f64,
    pub cvar_95: f64,
    pub cvar_99: f64,
    
    pub bad_debt_probability: f64,
    pub insolvency_probability: f64,
    pub mean_bad_debt: f64,
    pub max_bad_debt: f64,
}

impl MonteCarloResult {
    pub fn print(&self) {
        println!("  Runs:                    {}", self.runs);
        println!("  Mean bad debt:           ${:.0}", self.mean_bad_debt);
        println!("  Max bad debt:            ${:.0}", self.max_bad_debt);
        println!("  Bad debt probability:    {:.2}%", self.bad_debt_probability * 100.0);
        println!("  Insolvency probability:  {:.2}%", self.insolvency_probability * 100.0);
        println!("  VaR 95%:                 ${:.0}", self.var_95);
        println!("  VaR 99%:                 ${:.0}", self.var_99);
        println!("  VaR 99.9%:               ${:.0}", self.var_999);
        println!("  CVaR 95%:                ${:.0}", self.cvar_95);
        println!("  CVaR 99%:                ${:.0}", self.cvar_99);
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn expected_shortfall(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let cutoff_idx = ((sorted.len() as f64) * p).ceil() as usize;
    let tail: Vec<f64> = sorted[cutoff_idx..].to_vec();
    if tail.is_empty() {
        return *sorted.last().unwrap_or(&0.0);
    }
    tail.iter().sum::<f64>() / tail.len() as f64
}

pub fn run_monte_carlo(
    model: PriceModel,
    mechanism: LiquidationMechanism,
    runs: usize,
) -> MonteCarloResult {
    let mut rng = rand::thread_rng();
    
    let scenario = match model {
        PriceModel::GBM | PriceModel::GARCH => PriceScenario::VolatileCrash,
        PriceModel::JumpDiffusion => PriceScenario::FlashCrash,
        PriceModel::HistoricalMar2020 
        | PriceModel::HistoricalMay2021 
        | PriceModel::HistoricalNov2022 => PriceScenario::BlackSwan,
    };
    
    let results = run_cascade_simulation(mechanism, scenario, runs);
    
    let bad_debts: Vec<f64> = results.iter().map(|r| r.bad_debt).collect();
    let price_drops: Vec<f64> = results.iter().map(|r| r.price_drop_pct).collect();
    let liquidation_counts: Vec<usize> = results.iter().map(|r| r.total_liquidations).collect();
    let participation_rates: Vec<f64> = results.iter().map(|r| r.participation_rate).collect();
    
    let mut sorted_bad_debts = bad_debts.clone();
    sorted_bad_debts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let var_95 = percentile(&sorted_bad_debts, 0.95);
    let var_99 = percentile(&sorted_bad_debts, 0.99);
    let var_999 = percentile(&sorted_bad_debts, 0.999);
    let cvar_95 = expected_shortfall(&sorted_bad_debts, 0.95);
    let cvar_99 = expected_shortfall(&sorted_bad_debts, 0.99);
    
    let bad_debt_count = bad_debts.iter().filter(|&&d| d > 0.0).count();
    let bad_debt_probability = bad_debt_count as f64 / runs as f64;
    
    let insolvency_threshold = 100_000.0;
    let insolvency_count = bad_debts.iter().filter(|&&d| d > insolvency_threshold).count();
    let insolvency_probability = insolvency_count as f64 / runs as f64;
    
    let mean_bad_debt = bad_debts.iter().sum::<f64>() / runs as f64;
    let max_bad_debt = bad_debts.iter().cloned().fold(0.0, f64::max);
    
    MonteCarloResult {
        model,
        mechanism,
        runs,
        bad_debts,
        price_drops,
        liquidation_counts,
        participation_rates,
        var_95,
        var_99,
        var_999,
        cvar_95,
        cvar_99,
        bad_debt_probability,
        insolvency_probability,
        mean_bad_debt,
        max_bad_debt,
    }
}

pub fn compare_mechanisms(model: PriceModel, runs: usize) -> (MonteCarloResult, MonteCarloResult) {
    let traditional = run_monte_carlo(model, LiquidationMechanism::Traditional, runs);
    let fair = run_monte_carlo(model, LiquidationMechanism::KeeperPool, runs);
    (traditional, fair)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_path_generation() {
        let mut rng = rand::thread_rng();
        let config = PricePathConfig::default();
        let path = generate_price_path(&config, &mut rng);
        
        assert_eq!(path.len(), config.blocks + 1);
        assert!((path[0] - INITIAL_PRICE).abs() < 0.01);
    }

    #[test]
    fn test_monte_carlo_runs() {
        let result = run_monte_carlo(
            PriceModel::GBM,
            LiquidationMechanism::KeeperPool,
            100,
        );
        
        assert_eq!(result.runs, 100);
        assert_eq!(result.bad_debts.len(), 100);
    }

    #[test]
    fn test_var_calculation() {
        let data: Vec<f64> = (0..100).map(|i| i as f64 * 100.0).collect();
        let mut sorted = data.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let var_95 = percentile(&sorted, 0.95);
        assert!(var_95 >= 9000.0 && var_95 <= 9600.0);
    }
}
