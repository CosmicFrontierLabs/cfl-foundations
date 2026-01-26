//! Test to verify tracking coordinate consistency with ROI alignment constraints.
//!
//! This test reproduces issue #620 where TrackingStarted and TrackingUpdate
//! events may report different coordinates due to ROI alignment quantization.

use monocle::{
    callback::FgsCallbackEvent,
    config::{FgsConfig, GuideStarFilters},
    state::{FgsEvent, FgsState},
    FineGuidanceSystem,
};
use monocle_harness::{motion_profiles::StaticPointing, SimulatorCamera};
use shared::bad_pixel_map::BadPixelMap;
use shared::camera_interface::CameraInterface;
use simulator::hardware::{sensor::models::IMX455, SatelliteConfig, TelescopeConfig};
use simulator::units::{LengthExt, TemperatureExt};
use starfield::catalogs::binary_catalog::{BinaryCatalog, MinimalStar};
use std::sync::{Arc, Mutex};

/// Create a camera with IMX455-like ROI alignment constraints.
/// The IMX455 typically requires ROI offsets to be aligned to 32-pixel boundaries.
fn create_aligned_camera() -> SimulatorCamera {
    let telescope = TelescopeConfig::new(
        "Test Telescope",
        simulator::units::Length::from_meters(0.5),
        simulator::units::Length::from_meters(5.0),
        0.9,
    );

    // Use IMX455 sensor model with smaller dimensions for faster testing
    let sensor = IMX455.clone().with_dimensions(512, 512);

    let satellite = SatelliteConfig::new(
        telescope,
        sensor,
        simulator::units::Temperature::from_celsius(-10.0),
    );

    // Create catalog with stars at specific positions to test alignment
    // Use magnitude 10 stars to ensure they don't saturate
    // Point camera slightly off-center so star lands at non-aligned position
    let catalog = Arc::new(BinaryCatalog::from_stars(
        vec![
            // Star at RA/Dec = 0,0
            MinimalStar::new(1, 0.0, 0.0, 10.0),
        ],
        "Alignment test catalog",
    ));

    // Point slightly off so the star lands at a non-aligned pixel position
    // This should place the star around pixel (270, 270) instead of (256, 256)
    let motion = Box::new(StaticPointing::new(0.0003, 0.0003));
    let mut camera = SimulatorCamera::new(satellite, catalog, motion);

    // Set IMX455-like alignment constraints: 32 pixels for both H and V
    camera.set_roi_alignment(32, 32);

    camera
}

/// Create FGS config with alignment constraints matching the camera
fn create_fgs_config() -> FgsConfig {
    FgsConfig {
        acquisition_frames: 3,
        filters: GuideStarFilters {
            detection_threshold_sigma: 3.0,
            snr_min: 5.0,
            diameter_range: (2.0, 30.0),
            aspect_ratio_max: 3.0,
            saturation_value: 60000.0,
            saturation_search_radius: 5.0,
            minimum_edge_distance: 40.0, // Ensure star is away from edges
            bad_pixel_map: BadPixelMap::empty(),
            minimum_bad_pixel_distance: 5.0,
        },
        roi_size: 64, // 64x64 ROI
        max_reacquisition_attempts: 3,
        centroid_radius_multiplier: 3.0,
        fwhm: 3.0,
        snr_dropout_threshold: 3.0,
        roi_h_alignment: 32, // Match camera alignment
        roi_v_alignment: 32, // Match camera alignment
        noise_estimation_downsample: 1,
    }
}

#[test]
fn test_tracking_position_consistency_with_alignment() {
    // Enable logging
    let _ = env_logger::builder().is_test(true).try_init();
    // Collect tracking events
    let events: Arc<Mutex<Vec<FgsCallbackEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let events_clone = events.clone();

    let mut camera = create_aligned_camera();
    let config = create_fgs_config();
    let mut fgs = FineGuidanceSystem::new(config);

    // Register callback to collect events
    fgs.register_callback(move |event| {
        events_clone.lock().unwrap().push(event.clone());
    });

    // Start FGS
    let result = fgs.process_event(FgsEvent::StartFgs);
    println!("StartFgs result: {:?}", result);
    println!("State after StartFgs: {:?}", fgs.state());
    assert!(
        matches!(fgs.state(), FgsState::Acquiring { .. }),
        "Should be in Acquiring state, but got {:?}",
        fgs.state()
    );

    // Process frames for acquisition and tracking using capture_frame
    for frame_count in 1..=50 {
        let (frame, metadata) = camera.capture_frame().expect("Failed to capture frame");
        let result = fgs.process_frame(frame.view(), metadata.timestamp);

        match &result {
            Ok((_, camera_updates)) => {
                // Apply camera settings updates
                for update in camera_updates {
                    match update {
                        monocle::CameraSettingsUpdate::SetROI(roi) => {
                            println!("Setting ROI: {:?}", roi);
                            if let Err(e) = camera.set_roi(*roi) {
                                println!("Failed to set ROI: {}", e);
                            }
                        }
                        monocle::CameraSettingsUpdate::ClearROI => {
                            println!("Clearing ROI");
                            let _ = camera.clear_roi();
                        }
                    }
                }
            }
            Err(e) => {
                println!("Frame {} error: {}", frame_count, e);
            }
        }

        println!("Frame {}: state = {:?}", frame_count, fgs.state());

        // Stop after we've been tracking for a few frames
        if matches!(fgs.state(), FgsState::Tracking { frames_processed } if *frames_processed > 5) {
            break;
        }
    }

    // Analyze collected events
    let events = events.lock().unwrap();

    println!("Collected {} events:", events.len());
    for (i, event) in events.iter().enumerate() {
        match event {
            FgsCallbackEvent::TrackingStarted {
                initial_position, ..
            } => {
                println!(
                    "  [{}] TrackingStarted at ({:.2}, {:.2})",
                    i, initial_position.x, initial_position.y
                );
            }
            FgsCallbackEvent::TrackingUpdate { position, .. } => {
                println!(
                    "  [{}] TrackingUpdate at ({:.2}, {:.2})",
                    i, position.x, position.y
                );
            }
            FgsCallbackEvent::FrameProcessed { frame_number, .. } => {
                println!("  [{}] FrameProcessed #{}", i, frame_number);
            }
            FgsCallbackEvent::FrameSizeMismatch {
                expected_width,
                expected_height,
                actual_width,
                actual_height,
            } => {
                println!(
                    "  [{}] FrameSizeMismatch: expected {}x{}, got {}x{}",
                    i, expected_width, expected_height, actual_width, actual_height
                );
            }
            _ => {
                println!("  [{}] Other event: {:?}", i, std::mem::discriminant(event));
            }
        }
    }

    // Find TrackingStarted event
    let tracking_started = events.iter().find_map(|e| match e {
        FgsCallbackEvent::TrackingStarted {
            initial_position, ..
        } => Some(initial_position.clone()),
        _ => None,
    });

    // Find first TrackingUpdate event
    let first_tracking_update = events.iter().find_map(|e| match e {
        FgsCallbackEvent::TrackingUpdate { position, .. } => Some(position.clone()),
        _ => None,
    });

    // Both should exist
    let started_pos = tracking_started.expect("Should have TrackingStarted event");
    let update_pos = first_tracking_update.expect("Should have TrackingUpdate event");

    println!(
        "TrackingStarted position: ({:.2}, {:.2})",
        started_pos.x, started_pos.y
    );
    println!(
        "First TrackingUpdate position: ({:.2}, {:.2})",
        update_pos.x, update_pos.y
    );

    // Calculate the difference
    let dx = (update_pos.x - started_pos.x).abs();
    let dy = (update_pos.y - started_pos.y).abs();
    let distance = (dx * dx + dy * dy).sqrt();

    println!(
        "Position difference: dx={:.2}, dy={:.2}, distance={:.2}",
        dx, dy, distance
    );

    // The positions should be very close (within 1 pixel accounting for noise)
    // If there's a systematic offset due to ROI alignment issues, it will be larger
    assert!(
        distance < 2.0,
        "TrackingStarted and first TrackingUpdate positions differ by {:.2} pixels. \
         This may indicate ROI alignment is not properly accounted for. \
         Started: ({:.2}, {:.2}), Update: ({:.2}, {:.2})",
        distance,
        started_pos.x,
        started_pos.y,
        update_pos.x,
        update_pos.y
    );
}

#[test]
fn test_roi_alignment_offset_calculation() {
    // This test verifies the ROI alignment math directly
    use monocle::config::FgsConfig;

    let config = FgsConfig {
        acquisition_frames: 1,
        filters: GuideStarFilters {
            detection_threshold_sigma: 5.0,
            snr_min: 5.0,
            diameter_range: (2.0, 20.0),
            aspect_ratio_max: 2.5,
            saturation_value: 60000.0,
            saturation_search_radius: 3.0,
            minimum_edge_distance: 40.0,
            bad_pixel_map: BadPixelMap::empty(),
            minimum_bad_pixel_distance: 5.0,
        },
        roi_size: 64,
        max_reacquisition_attempts: 3,
        centroid_radius_multiplier: 3.0,
        fwhm: 3.0,
        snr_dropout_threshold: 3.0,
        roi_h_alignment: 32,
        roi_v_alignment: 32,
        noise_estimation_downsample: 1,
    };

    // Test various star positions and verify ROI alignment
    let test_cases = vec![
        // (star_x, star_y, description)
        (270.5, 270.5, "Star at non-aligned position"),
        (256.0, 256.0, "Star at aligned position"),
        (280.3, 290.7, "Star with fractional position"),
        (100.0, 400.0, "Star near corner"),
    ];

    for (star_x, star_y, desc) in test_cases {
        let roi = config
            .compute_aligned_roi(star_x, star_y, 512, 512)
            .expect(&format!("Should compute ROI for {}", desc));

        // Verify alignment
        assert_eq!(
            roi.min_col % 32,
            0,
            "{}: ROI min_col {} should be aligned to 32",
            desc,
            roi.min_col
        );
        assert_eq!(
            roi.min_row % 32,
            0,
            "{}: ROI min_row {} should be aligned to 32",
            desc,
            roi.min_row
        );

        // Calculate star position within ROI
        let star_in_roi_x = star_x - roi.min_col as f64;
        let star_in_roi_y = star_y - roi.min_row as f64;

        // Star should still be within the ROI bounds
        assert!(
            star_in_roi_x >= 0.0 && star_in_roi_x < 64.0,
            "{}: Star x position {:.2} should be within ROI (0-64)",
            desc,
            star_in_roi_x
        );
        assert!(
            star_in_roi_y >= 0.0 && star_in_roi_y < 64.0,
            "{}: Star y position {:.2} should be within ROI (0-64)",
            desc,
            star_in_roi_y
        );

        // The star should be reasonably close to the center of the ROI
        // Maximum offset from center should be alignment/2 = 16 pixels
        let center_offset_x = (star_in_roi_x - 32.0).abs();
        let center_offset_y = (star_in_roi_y - 32.0).abs();

        println!(
            "{}: star at ({:.1}, {:.1}), ROI at ({}, {}), star in ROI at ({:.1}, {:.1}), center offset: ({:.1}, {:.1})",
            desc, star_x, star_y, roi.min_col, roi.min_row, star_in_roi_x, star_in_roi_y, center_offset_x, center_offset_y
        );

        // With 32-pixel alignment and round-to-nearest, max offset should be ~16 pixels
        assert!(
            center_offset_x <= 17.0,
            "{}: Star x offset from ROI center {:.2} exceeds expected maximum",
            desc,
            center_offset_x
        );
        assert!(
            center_offset_y <= 17.0,
            "{}: Star y offset from ROI center {:.2} exceeds expected maximum",
            desc,
            center_offset_y
        );
    }
}
