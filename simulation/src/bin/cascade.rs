//! Deleveraging Cascade Simulation Binary
//!
//! Compares Fair's keeper pool mechanism vs traditional winner-takes-all
//! under various stress scenarios.
//!
//! ## Usage
//! ```bash
//! cargo run --bin cascade --release
//! ```

use fair_simulation::cascade::{
    run_cascade_simulation, aggregate_results,
    LiquidationMechanism, PriceScenario,
};

const SIMULATION_RUNS: usize = 1000;

fn main() {
    println!("=======================================================");
    println!("  Deleveraging Cascade Simulation");
    println!("  Comparing Fair vs Traditional Liquidation");
    println!("=======================================================");
    println!();
    println!("Parameters:");
    println!("  CDPs: 500, Keepers: 50, Runs: {}", SIMULATION_RUNS);
    println!("  Liquidations per block: 10");
    println!("  Price impact: 0.01% per ETH sold");
    println!();

    for scenario in PriceScenario::all() {
        println!("=======================================================");
        println!("Scenario: {}", scenario.name());
        println!("=======================================================");
        println!();

        for mechanism in LiquidationMechanism::all() {
            println!("Mechanism: {}", mechanism.name());
            println!("{}", "-".repeat(50));

            let results = run_cascade_simulation(mechanism, scenario, SIMULATION_RUNS);
            let agg = aggregate_results(&results);
            agg.print();
            println!();
        }
    }

    println!("=======================================================");
    println!("  Summary: Fair vs Traditional");
    println!("=======================================================");
    println!();
    
    print_comparison_table();
}

fn print_comparison_table() {
    println!("| Scenario            | Mechanism   | Bad Debt | Participation | Concentration |");
    println!("|---------------------|-------------|----------|---------------|---------------|");

    for scenario in PriceScenario::all() {
        for mechanism in LiquidationMechanism::all() {
            let results = run_cascade_simulation(mechanism, scenario, 100);
            let agg = aggregate_results(&results);
            
            let scenario_name = match scenario {
                PriceScenario::GradualDecline => "Gradual",
                PriceScenario::FlashCrash => "Flash",
                PriceScenario::VolatileCrash => "Volatile",
                PriceScenario::BlackSwan => "Black Swan",
            };
            
            let mech_name = match mechanism {
                LiquidationMechanism::Traditional => "Traditional",
                LiquidationMechanism::KeeperPool => "Fair",
            };
            
            println!(
                "| {:19} | {:11} | ${:6.0} | {:12.1}% | {:12.1}% |",
                scenario_name,
                mech_name,
                agg.avg_bad_debt,
                agg.avg_participation_rate * 100.0,
                agg.avg_profit_concentration * 100.0,
            );
        }
    }
}
