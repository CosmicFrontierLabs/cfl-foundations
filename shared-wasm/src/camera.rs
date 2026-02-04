//! Camera and sensor data types.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Pipeline timing statistics from camera_server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CameraTimingStats {
    pub avg_capture_ms: f32,
    pub avg_analysis_ms: f32,
    pub avg_render_ms: f32,
    pub avg_total_pipeline_ms: f32,
    pub capture_samples: usize,
    pub analysis_samples: usize,
}

/// Camera statistics from camera_server /stats endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CameraStats {
    pub total_frames: u64,
    pub avg_fps: f32,
    pub temperatures: HashMap<String, f64>,
    pub histogram: Vec<u32>,
    pub histogram_mean: f64,
    pub histogram_max: u16,
    /// Pipeline timing info (optional, frontend may ignore)
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timing: Option<CameraTimingStats>,
    /// Camera device name
    pub device_name: String,
    /// Camera resolution width in pixels
    pub width: u32,
    /// Camera resolution height in pixels
    pub height: u32,
}

/// Raw frame response from camera_server /raw endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RawFrameResponse {
    pub width: usize,
    pub height: usize,
    pub timestamp_sec: u64,
    pub timestamp_nanos: u64,
    pub temperatures: HashMap<String, f64>,
    pub exposure_us: u128,
    pub frame_number: u64,
    pub image_base64: String,
}
