//! Monte Carlo Stress Testing Binary
//!
//! Runs statistical analysis of Fair vs Traditional under various price models.
//!
//! ## Usage
//! ```bash
//! cargo run --bin monte_carlo --release
//! ```

use fair_simulation::monte_carlo::{run_monte_carlo, compare_mechanisms, PriceModel};
use fair_simulation::cascade::LiquidationMechanism;

const SIMULATION_RUNS: usize = 10_000;

fn main() {
    println!("=======================================================");
    println!("  Monte Carlo Stress Testing");
    println!("  Statistical Analysis of Fair Stablecoin");
    println!("=======================================================");
    println!();
    println!("Parameters:");
    println!("  Runs per scenario: {}", SIMULATION_RUNS);
    println!("  CDPs: 500, Keepers: 50");
    println!();

    for model in PriceModel::all() {
        println!("=======================================================");
        println!("Price Model: {}", model.name());
        println!("=======================================================");
        println!();

        let (trad, fair) = compare_mechanisms(model, SIMULATION_RUNS);

        println!("Mechanism: Traditional (Winner-Takes-All)");
        println!("{}", "-".repeat(50));
        trad.print();
        println!();

        println!("Mechanism: Fair (Keeper Pool 70/30)");
        println!("{}", "-".repeat(50));
        fair.print();
        println!();

        let improvement = if trad.mean_bad_debt > 0.0 {
            (1.0 - fair.mean_bad_debt / trad.mean_bad_debt) * 100.0
        } else if fair.mean_bad_debt > 0.0 {
            -100.0
        } else {
            0.0
        };

        println!("Comparison:");
        println!("  Bad debt improvement:    {:.1}%", improvement);
        println!(
            "  VaR 99% ratio:           {:.2}x",
            if trad.var_99 > 0.0 { fair.var_99 / trad.var_99 } else { 0.0 }
        );
        println!(
            "  Insolvency prob ratio:   {:.2}x",
            if trad.insolvency_probability > 0.0 {
                fair.insolvency_probability / trad.insolvency_probability
            } else {
                0.0
            }
        );
        println!();
    }

    println!("=======================================================");
    println!("  Summary Table");
    println!("=======================================================");
    println!();
    print_summary_table();
}

fn print_summary_table() {
    println!("| Model            | Mechanism   | Mean Debt | VaR 99% | P(Insolvency) |");
    println!("|------------------|-------------|-----------|---------|---------------|");

    for model in PriceModel::all() {
        let (trad, fair) = compare_mechanisms(model, 1000);

        let model_name = match model {
            PriceModel::GBM => "GBM",
            PriceModel::JumpDiffusion => "Jump-Diff",
            PriceModel::GARCH => "GARCH",
            PriceModel::HistoricalMar2020 => "Mar 2020",
            PriceModel::HistoricalMay2021 => "May 2021",
            PriceModel::HistoricalNov2022 => "Nov 2022",
        };

        println!(
            "| {:16} | {:11} | ${:7.0} | ${:6.0} | {:12.1}% |",
            model_name, "Traditional", trad.mean_bad_debt, trad.var_99, trad.insolvency_probability * 100.0
        );
        println!(
            "| {:16} | {:11} | ${:7.0} | ${:6.0} | {:12.1}% |",
            "", "Fair", fair.mean_bad_debt, fair.var_99, fair.insolvency_probability * 100.0
        );
    }
}
