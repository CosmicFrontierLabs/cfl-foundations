//! Calibration display configuration types.

use serde::{Deserialize, Serialize};

/// Current pattern configuration from calibrate_serve.
///
/// Returned via HTTP GET /config endpoint. Used by frontend to sync
/// state with server (e.g., after ZMQ commands or idle timeout).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatternConfigResponse {
    /// Pattern type identifier (e.g., "Crosshair", "Uniform")
    pub pattern_id: String,
    /// Pattern-specific parameter values (JSON object)
    pub values: serde_json::Value,
    /// Whether pattern colors are inverted
    pub invert: bool,
}

/// Display system information from calibrate_serve.
///
/// Returned via HTTP GET /info endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DisplayInfo {
    /// Display width in pixels
    pub width: u32,
    /// Display height in pixels
    pub height: u32,
    /// Pixel pitch in microns (None if unknown/unavailable)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub pixel_pitch_um: Option<f64>,
    /// Display name/identifier (e.g., "OLED 2560x2560")
    pub name: String,
}

/// Control specification for the frontend UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ControlSpec {
    IntRange {
        id: String,
        label: String,
        min: i64,
        max: i64,
        step: i64,
        default: i64,
    },
    FloatRange {
        id: String,
        label: String,
        min: f64,
        max: f64,
        step: f64,
        default: f64,
    },
    Bool {
        id: String,
        label: String,
        default: bool,
    },
    Text {
        id: String,
        label: String,
        default: String,
        placeholder: String,
    },
}

/// Pattern specification for the frontend UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PatternSpec {
    pub id: String,
    pub name: String,
    pub controls: Vec<ControlSpec>,
}

/// Schema response containing all patterns and global controls.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchemaResponse {
    pub patterns: Vec<PatternSpec>,
    pub global_controls: Vec<ControlSpec>,
}
