//! Star detection settings and results.

use serde::{Deserialize, Serialize};

/// Settings for real-time star detection overlay.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StarDetectionSettings {
    /// Enable star detection overlay
    pub enabled: bool,
    /// Detection threshold in sigma above background (3.0 - 20.0)
    pub detection_sigma: f64,
    /// Minimum distance from bad pixels to consider valid (pixels)
    pub min_bad_pixel_distance: f64,
    /// Maximum aspect ratio for valid star (rejects elongated artifacts)
    pub max_aspect_ratio: f64,
    /// Minimum flux for valid detection
    pub min_flux: f64,
}

impl Default for StarDetectionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            detection_sigma: 5.0,
            min_bad_pixel_distance: 5.0,
            max_aspect_ratio: 2.5,
            min_flux: 100.0,
        }
    }
}

/// A detected star with position and shape info.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectedStar {
    /// X position (sub-pixel)
    pub x: f64,
    /// Y position (sub-pixel)
    pub y: f64,
    /// Total flux
    pub flux: f64,
    /// Aspect ratio (1.0 = circular)
    pub aspect_ratio: f64,
    /// Estimated diameter in pixels
    pub diameter: f64,
    /// Whether star passes quality filters (not near bad pixel, good shape)
    pub valid: bool,
}

/// Star detection result for a frame.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StarDetectionResult {
    /// Detected stars
    pub stars: Vec<DetectedStar>,
    /// Detection time in milliseconds
    pub detection_time_ms: f32,
    /// Frame number this detection applies to
    pub frame_number: u64,
}
