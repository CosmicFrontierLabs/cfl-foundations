//! Shared WASM-compatible types used across multiple repositories.
//!
//! Contains types that are genuinely shared between cfl-foundations consumers
//! (meter-sim, focalplane). All types must be WASM-compatible.

pub mod stats_scan;
mod types;

pub use stats_scan::{StatsError, StatsScan};
pub use types::{SpotShape, Timestamp};
