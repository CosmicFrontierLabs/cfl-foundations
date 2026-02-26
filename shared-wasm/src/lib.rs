//! Shared types for test-bench backend and frontend.
//!
//! This crate contains lightweight serialization types that can be used
//! by both the Rust backend (test-bench) and WASM frontend (test-bench-frontend).
//! All types here must be WASM-compatible (no threading, no C bindings).

mod calibrate_client;
mod calibration;
mod camera;
mod http_client;
mod log;
mod pattern;
mod star_detection;
pub mod stats_scan;
mod tracking;
mod types;

pub use calibrate_client::{CalibrateError, CalibrateServerClient, PatternConfigRequest};
pub use calibration::{
    ControlSpec, DisplayInfo, PatternConfigResponse, PatternSpec, SchemaResponse,
};
pub use camera::{CameraStats, CameraTimingStats, RawFrameResponse};
pub use http_client::HttpClientError;
pub use log::{LogEntry, LogLevel};
pub use pattern::{generate_centered_grid, PatternCommand};
pub use star_detection::{DetectedStar, StarDetectionResult, StarDetectionSettings};
pub use stats_scan::{StatsError, StatsScan};
pub use tracking::{
    TrackingEnableRequest, TrackingPosition, TrackingSettings, TrackingState, TrackingStatus,
};
pub use types::{HealthInfo, SpotShape, Timestamp};

use serde::{Deserialize, Serialize};

/// Error from a WebSocket command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandError {
    /// Which command failed
    pub command: String,
    /// Human-readable error message
    pub message: String,
}
