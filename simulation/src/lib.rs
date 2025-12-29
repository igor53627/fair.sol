//! Fair Stablecoin Simulation Library
//!
//! This library provides simulation tools for analyzing the Fair stablecoin's
//! liquidation mechanism, comparing it against traditional approaches.
//!
//! ## Modules
//!
//! - `poa`: Price of Anarchy simulation (single-shot liquidation game)
//! - `cascade`: Deleveraging cascade simulation (multi-step dynamics)
//! - `monte_carlo`: Monte Carlo stress testing with VaR/CVaR metrics
//!
//! ## Usage
//!
//! ```bash
//! # Run Price of Anarchy simulation
//! cargo run --bin poa --release
//!
//! # Run Cascade simulation
//! cargo run --bin cascade --release
//!
//! # Run Monte Carlo stress testing
//! cargo run --bin monte_carlo --release
//! ```

pub mod poa;
pub mod cascade;
pub mod monte_carlo;
