//! Monocle harness for testing and simulation
//!
//! This module provides test harnesses and simulation infrastructure
//! for the monocle fine guidance system. It bridges the simulator
//! and monocle modules for testing and demonstration purposes.

pub mod simulator_camera;
pub mod test_motions;
pub mod tracking_plots;

pub use simulator_camera::SimulatorCamera;
pub use test_motions::TestMotions;
pub use tracking_plots::{TrackingDataPoint, TrackingPlotConfig, TrackingPlotter};
