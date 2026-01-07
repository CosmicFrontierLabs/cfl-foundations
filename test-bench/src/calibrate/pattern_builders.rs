use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use shared::image_size::PixelShape;
use shared::tracking_collector::TrackingCollector;

use crate::display_patterns as patterns;

use super::pattern::PatternConfig;

/// Load motion profile pattern from image and CSV files.
pub fn load_motion_profile(
    image_path: &PathBuf,
    csv_path: &PathBuf,
    width: u32,
    height: u32,
    motion_scale: f64,
) -> Result<PatternConfig> {
    let base_image =
        patterns::motion_profile::load_and_downsample_image(image_path, width, height)?;
    let motion_profile = patterns::motion_profile::load_motion_profile(csv_path)?;
    Ok(PatternConfig::MotionProfile {
        base_image,
        motion_profile,
        motion_scale,
    })
}

/// Create gyro walk pattern from image file.
pub fn create_gyro_walk(
    image_path: &PathBuf,
    width: u32,
    height: u32,
    pixel_size_um: f64,
    focal_length_mm: f64,
    motion_scale: f64,
    frame_rate_hz: f64,
) -> Result<PatternConfig> {
    let base_image =
        patterns::motion_profile::load_and_downsample_image(image_path, width, height)?;
    let gyro_state =
        patterns::gyro_walk::GyroWalkState::new(pixel_size_um, focal_length_mm, motion_scale);
    Ok(PatternConfig::GyroWalk {
        base_image,
        gyro_state: Arc::new(Mutex::new(gyro_state)),
        frame_rate_hz,
    })
}

/// Create optical calibration pattern with ZMQ feedback.
pub fn create_optical_calibration(
    zmq_endpoint: &str,
    grid_size: usize,
    grid_spacing: f64,
    pattern_width: u32,
    pattern_height: u32,
    spot_fwhm: f64,
    warmup_duration: Duration,
) -> Result<PatternConfig> {
    let collector = TrackingCollector::connect(zmq_endpoint)
        .map_err(|e| anyhow::anyhow!("Failed to connect ZMQ subscriber: {e}"))?;

    let pattern_size =
        PixelShape::with_width_height(pattern_width as usize, pattern_height as usize);
    let runner = patterns::optical_calibration::CalibrationRunner::for_grid(
        collector,
        grid_size,
        grid_spacing,
        pattern_size,
        spot_fwhm,
        warmup_duration,
    );

    Ok(PatternConfig::OpticalCalibration {
        runner: Arc::new(Mutex::new(runner)),
        pattern_size,
    })
}
