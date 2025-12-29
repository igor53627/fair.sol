//! IPFE Price of Anarchy Simulation Binary
//!
//! Compares obfuscation strategies for stablecoin liquidation.
//!
//! ## Usage
//! ```bash
//! cargo run --bin poa --release
//! ```

use fair_simulation::poa::{run_poa_simulation, compute_poa, ObfuscationStrategy};

const SIMULATION_RUNS: usize = 10_000;

fn main() {
    println!("=======================================================");
    println!("  IPFE Price of Anarchy Simulation");
    println!("  Comparing obfuscation strategies for liquidation");
    println!("=======================================================\n");

    for strategy in ObfuscationStrategy::all() {
        println!("Strategy: {}", strategy.name());
        println!("{}", "-".repeat(50));

        let results = run_poa_simulation(strategy, SIMULATION_RUNS);
        let poa = compute_poa(&results);

        let avg_successful: f64 = results
            .iter()
            .map(|r| r.successful_liquidations as f64)
            .sum::<f64>()
            / SIMULATION_RUNS as f64;

        let avg_failed: f64 = results.iter().map(|r| r.failed_attempts as f64).sum::<f64>()
            / SIMULATION_RUNS as f64;

        let avg_missed: f64 = results
            .iter()
            .map(|r| r.missed_liquidations as f64)
            .sum::<f64>()
            / SIMULATION_RUNS as f64;

        let avg_concentration: f64 = results.iter().map(|r| r.profit_concentration).sum::<f64>()
            / SIMULATION_RUNS as f64;

        let front_runner_share: f64 = results
            .iter()
            .map(|r| {
                if r.total_profit > 0.0 {
                    r.front_runner_profit / r.total_profit
                } else {
                    0.0
                }
            })
            .sum::<f64>()
            / SIMULATION_RUNS as f64;

        println!("  Successful liquidations: {:.1}", avg_successful);
        println!("  Failed attempts:         {:.1}", avg_failed);
        println!("  Missed (bad debt risk):  {:.1}", avg_missed);
        println!("  Profit concentration:    {:.1}%", avg_concentration * 100.0);
        println!("  Front-runner share:      {:.1}%", front_runner_share * 100.0);
        println!("  Price of Anarchy:        {:.2}", poa);
        println!();
    }

    println!("=======================================================");
    println!("  Interpretation:");
    println!("  - PoA = 1.0 means fair, efficient market");
    println!("  - PoA > 1.0 means value extraction by sophisticated actors");
    println!("  - Lower PoA = better for protocol health");
    println!("=======================================================");
}
