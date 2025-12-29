//! Fair Stablecoin Simulation Library
//!
//! This library provides simulation tools for analyzing the Fair stablecoin's
//! liquidation mechanism, comparing it against traditional approaches.
//!
//! ## Modules
//!
//! - `poa`: Price of Anarchy simulation (single-shot liquidation game)
//! - `cascade`: Deleveraging cascade simulation (multi-step dynamics)
//!
//! ## Usage
//!
//! ```bash
//! # Run Price of Anarchy simulation
//! cargo run --bin poa --release
//!
//! # Run Cascade simulation
//! cargo run --bin cascade --release
//! ```

pub mod poa;
pub mod cascade;
