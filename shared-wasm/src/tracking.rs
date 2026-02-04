//! Tracking system types.

use serde::{Deserialize, Serialize};

/// Tracking state enum for the camera unified server.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TrackingState {
    /// System is idle, not tracking
    #[default]
    Idle,
    /// Acquiring frames to detect guide stars
    Acquiring { frames_collected: usize },
    /// Calibrating detected guide stars
    Calibrating,
    /// Actively tracking targets
    Tracking { frames_processed: usize },
    /// Lost track, attempting to reacquire
    Reacquiring { attempts: usize },
}

/// Current tracking position information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackingPosition {
    /// Current track ID
    pub track_id: u32,
    /// X position in pixels
    pub x: f64,
    /// Y position in pixels
    pub y: f64,
    /// Signal-to-noise ratio of tracked target
    pub snr: f64,
    /// Timestamp of position measurement (seconds since epoch)
    pub timestamp_sec: u64,
    /// Nanoseconds component of timestamp
    pub timestamp_nanos: u64,
}

/// Full tracking status response from /tracking/status endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackingStatus {
    /// Whether tracking mode is enabled
    pub enabled: bool,
    /// Current tracking state
    pub state: TrackingState,
    /// Current tracked position (if tracking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<TrackingPosition>,
    /// Number of guide stars being tracked
    pub num_guide_stars: usize,
    /// Total tracking updates since tracking started
    pub total_updates: u64,
}

impl Default for TrackingStatus {
    fn default() -> Self {
        Self {
            enabled: false,
            state: TrackingState::Idle,
            position: None,
            num_guide_stars: 0,
            total_updates: 0,
        }
    }
}

/// Request to enable/disable tracking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackingEnableRequest {
    pub enabled: bool,
}

/// Tracking algorithm settings that can be adjusted at runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrackingSettings {
    /// Number of frames to collect during acquisition phase
    pub acquisition_frames: usize,
    /// Size of ROI around detected stars (pixels)
    pub roi_size: usize,
    /// Detection threshold in standard deviations above background
    pub detection_threshold_sigma: f64,
    /// Minimum SNR required to start tracking a star
    pub snr_min: f64,
    /// SNR threshold below which tracking is considered lost
    pub snr_dropout_threshold: f64,
    /// Expected Full Width at Half Maximum of stars (pixels)
    pub fwhm: f64,
}

impl Default for TrackingSettings {
    fn default() -> Self {
        Self {
            acquisition_frames: 5,
            roi_size: 64,
            detection_threshold_sigma: 5.0,
            snr_min: 10.0,
            snr_dropout_threshold: 3.0,
            fwhm: 7.0,
        }
    }
}

/// Export settings for tracking data recording.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExportSettings {
    /// Enable CSV export of tracking data
    pub csv_enabled: bool,
    /// CSV output filename (relative to working directory)
    pub csv_filename: String,
    /// Enable frame export as PNG + JSON metadata
    pub frames_enabled: bool,
    /// Directory for frame export (relative to working directory)
    pub frames_directory: String,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            csv_enabled: false,
            csv_filename: "tracking_data.csv".to_string(),
            frames_enabled: false,
            frames_directory: "frames".to_string(),
        }
    }
}

/// Export status showing current export statistics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ExportStatus {
    /// Number of CSV records written
    pub csv_records_written: u64,
    /// Number of frames exported
    pub frames_exported: u64,
    /// Current export settings
    pub settings: ExportSettings,
    /// Last export error (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// Metadata for an exported frame (written alongside PNG files).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FrameExportMetadata {
    /// Frame sequence number
    pub frame_number: usize,
    /// Timestamp seconds component
    pub timestamp_sec: u64,
    /// Timestamp nanoseconds component
    pub timestamp_nanos: u64,
    /// Current track ID (if tracking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_id: Option<u32>,
    /// Centroid X position (if tracking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub centroid_x: Option<f64>,
    /// Centroid Y position (if tracking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub centroid_y: Option<f64>,
    /// Frame width in pixels
    pub width: usize,
    /// Frame height in pixels
    pub height: usize,
}
