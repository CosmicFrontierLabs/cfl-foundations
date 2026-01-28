use clap::Parser;
use monocle::{
    callback::FgsCallbackEvent,
    config::FgsConfig,
    state::{FgsEvent, FgsState},
    FineGuidanceSystem,
};
use monocle_harness::{
    create_guide_star_catalog, create_jbt_hwk_camera_with_catalog_and_motion,
    motion_profiles::{PointingMotion, TestMotions},
    simulator_camera::SimulatorCamera,
    tracking_plots::{TrackingDataPoint, TrackingPlotConfig, TrackingPlotter},
};
use shared::camera_interface::CameraInterface;
use shared::star_projector::StarProjector;
use shared::units::AngleExt;
use starfield::Equatorial;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Command line arguments for tracking demo
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "FGS tracking demonstration with motion patterns",
    long_about = "Demonstrates FGS tracking performance under various motion patterns.\n\n\
        This tool runs a simulated FGS tracking session with configurable motion profiles \
        and generates plots showing tracking accuracy. Useful for:\n  \
        - Visualizing FGS behavior under different motion types\n  \
        - Tuning FGS parameters for specific motion amplitudes\n  \
        - Generating tracking performance plots for documentation\n\n\
        Output plots are saved to the plots/ directory."
)]
struct Args {
    #[arg(
        short,
        long,
        default_value = "sine_ra",
        help = "Motion pattern type",
        long_help = "Type of motion pattern to simulate. Available patterns:\n  \
            - stationary: No motion (static pointing)\n  \
            - sine_ra: Sinusoidal oscillation in right ascension\n  \
            - sine_dec: Sinusoidal oscillation in declination\n  \
            - circular: Circular motion combining RA and Dec\n  \
            - chaotic: Random jitter motion\n\n\
            Motion amplitude is controlled by --amplitude."
    )]
    motion: String,

    #[arg(
        short = 't',
        long,
        default_value_t = 10.0,
        help = "Simulation duration in seconds",
        long_help = "Total duration of the tracking simulation in seconds. Longer durations \
            show more motion cycles but increase runtime. The plot x-axis spans from 0 \
            to this duration. Typical range: 5-60 seconds."
    )]
    duration: f64,

    #[arg(
        short,
        long,
        help = "Output filename for the plot (without path)",
        long_help = "Filename for the output plot PNG file. Saved to plots/ directory. \
            If not specified, defaults to 'tracking_{motion}.png' where {motion} is \
            the motion pattern name. A residuals plot is also generated with '_residuals' \
            suffix."
    )]
    output: Option<String>,

    #[arg(
        long,
        default_value_t = 2400,
        help = "Plot width in pixels",
        long_help = "Width of the output plot image in pixels. Higher values produce \
            sharper plots but larger files. Typical range: 1200-3600 pixels."
    )]
    width: u32,

    #[arg(
        long,
        default_value_t = 1600,
        help = "Plot height in pixels",
        long_help = "Height of the output plot image in pixels. Higher values produce \
            sharper plots but larger files. Typical range: 800-2400 pixels."
    )]
    height: u32,

    #[arg(
        long,
        default_value_t = 3,
        help = "Number of frames for FGS acquisition phase",
        long_help = "Number of full-frame images captured during the FGS acquisition phase \
            before selecting a guide star. More frames improve detection reliability \
            but increase time to first lock. Typical range: 2-5 frames."
    )]
    acquisition_frames: usize,

    #[arg(
        long,
        default_value_t = 10.0,
        help = "Minimum SNR for guide star selection",
        long_help = "Minimum signal-to-noise ratio required for a star to be selected \
            as a guide star. Higher values ensure more reliable centroids. With the \
            synthetic star catalog used in this demo, 10.0 is typically sufficient."
    )]
    min_snr: f64,

    #[arg(
        long,
        default_value_t = 32,
        help = "ROI size in pixels (square)",
        long_help = "Size of the region-of-interest window used for tracking, in pixels. \
            Must be large enough to contain the PSF with margin for motion amplitude. \
            For this demo's typical amplitudes, 32-64 pixels is sufficient."
    )]
    roi_size: usize,

    #[arg(
        long,
        default_value_t = 5.0,
        help = "Centroid aperture radius as multiple of FWHM",
        long_help = "Radius of the centroiding aperture expressed as a multiple of the \
            PSF full-width at half-maximum. Larger apertures capture more of the PSF \
            but include more background noise. Typical range: 3.0-6.0."
    )]
    centroid_multiplier: f64,

    #[arg(
        long,
        default_value_t = 10.0,
        help = "Simulated frame rate in Hz",
        long_help = "Frame rate for the simulation in frames per second. Higher rates \
            produce more data points but increase computation time. Typical rates: \
            10-30 Hz for guide star tracking, 1-5 Hz for slow motions."
    )]
    frame_rate: f64,

    #[arg(
        short = 'a',
        long,
        default_value_t = 10.0,
        help = "Motion amplitude in arcseconds",
        long_help = "Peak amplitude of the motion pattern in arcseconds. For sinusoidal \
            patterns, this is the peak-to-zero amplitude (full swing is 2x this value). \
            Larger amplitudes test the FGS ability to track fast-moving targets. \
            Typical range: 1-100 arcseconds."
    )]
    amplitude: f64,

    #[arg(
        short,
        long,
        help = "Enable verbose output",
        long_help = "Print detailed information during tracking including position updates, \
            lock status changes, and frame-by-frame progress. Useful for debugging \
            but produces a lot of output."
    )]
    verbose: bool,
}

/// Create a simulator camera with test configuration and motion
fn create_test_camera_with_motion(
    pointing: Equatorial,
    motion: Box<dyn PointingMotion>,
) -> SimulatorCamera {
    let catalog = Arc::new(create_guide_star_catalog(&pointing));
    create_jbt_hwk_camera_with_catalog_and_motion(catalog, motion)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let args = Args::parse();

    println!("FGS Tracking Demo");
    println!("=================");
    println!("Motion type: {}", args.motion);
    println!("Duration: {} seconds", args.duration);
    println!("Frame rate: {} Hz", args.frame_rate);

    // Setup motion pattern
    let base_pointing = Equatorial::from_degrees(83.0, -5.0);
    let test_motions = TestMotions::new(83.0, -5.0);
    let motion = test_motions
        .get_motion_with_amplitude(&args.motion, args.amplitude)
        .ok_or_else(|| format!("Unknown motion type: {}", args.motion))?;

    // Clone motion for tracking actual positions during callbacks
    let motion_for_tracking = motion.clone();

    // Create camera with motion profile
    let mut camera = create_test_camera_with_motion(base_pointing, motion);
    let frame_interval_ms = (1000.0 / args.frame_rate) as u64;
    camera.set_exposure(Duration::from_millis(frame_interval_ms))?;

    // Get actual sensor dimensions and telescope config for accurate projection
    let satellite_config = camera.satellite_config().clone();
    let (sensor_width, sensor_height) = satellite_config.sensor.dimensions.get_pixel_width_height();

    // Get pixel scale from satellite configuration
    let radians_per_pixel = satellite_config.plate_scale_per_pixel().as_radians();

    // Create FGS with configuration and camera
    // Get ROI alignment from camera (SimulatorCamera uses default 1,1)
    let (roi_h_alignment, roi_v_alignment) = camera.get_roi_offset_alignment();

    let mut config = FgsConfig {
        acquisition_frames: args.acquisition_frames,
        filters: monocle::config::GuideStarFilters {
            detection_threshold_sigma: 5.0,
            snr_min: args.min_snr,
            diameter_range: (2.0, 20.0),
            aspect_ratio_max: 2.5,
            saturation_value: 4000.0,
            saturation_search_radius: 3.0,
            minimum_edge_distance: 10.0,
            bad_pixel_map: shared::bad_pixel_map::BadPixelMap::empty(),
            minimum_bad_pixel_distance: 5.0,
        },
        roi_size: args.roi_size,
        max_reacquisition_attempts: 3,
        centroid_radius_multiplier: args.centroid_multiplier,
        fwhm: 3.0,
        snr_dropout_threshold: 3.0,
        noise_estimation_downsample: 16,
    };

    // Update config with camera's saturation value (95% of max to be conservative)
    config.filters.saturation_value = camera.saturation_value() * 0.95;

    let mut fgs = FineGuidanceSystem::new(config, (roi_h_alignment, roi_v_alignment));

    // Determine output filename
    let output_filename = args
        .output
        .unwrap_or_else(|| format!("tracking_{}.png", args.motion));

    // Create tracking plotter
    let mut plotter = TrackingPlotter::with_config(TrackingPlotConfig {
        output_filename: output_filename.clone(),
        title: format!("FGS Tracking: {} Motion", args.motion),
        width: args.width,
        height: args.height,
        max_time_seconds: args.duration,
    });

    // Track state for plotting
    let lock_established = Arc::new(Mutex::new(false));
    let lock_clone = lock_established.clone();
    let center_x = sensor_width as f64 / 2.0;
    let center_y = sensor_height as f64 / 2.0;
    let estimated_position = Arc::new(Mutex::new((center_x, center_y)));
    let position_clone = estimated_position.clone();

    // Register callback to track events
    let verbose = args.verbose;
    let _callback_id = fgs.register_callback(move |event| match event {
        FgsCallbackEvent::TrackingStarted {
            initial_position, ..
        } => {
            if verbose {
                println!(
                    "Tracking started at ({:.1}, {:.1})",
                    initial_position.x, initial_position.y
                );
            }
            *lock_clone.lock().unwrap() = true;
            *position_clone.lock().unwrap() = (initial_position.x, initial_position.y);
        }
        FgsCallbackEvent::TrackingUpdate { position, .. } => {
            if verbose {
                println!(
                    "Tracking update: pos=({:.1}, {:.1})",
                    position.x, position.y
                );
            }
            *position_clone.lock().unwrap() = (position.x, position.y);
        }
        FgsCallbackEvent::TrackingLost { .. } => {
            if verbose {
                println!("Tracking lost!");
            }
            *lock_clone.lock().unwrap() = false;
        }
        FgsCallbackEvent::FrameSizeMismatch {
            expected_width,
            expected_height,
            actual_width,
            actual_height,
        } => {
            if verbose {
                println!("Frame size mismatch: expected {expected_width}x{expected_height}, got {actual_width}x{actual_height}");
            }
        }
        FgsCallbackEvent::FrameProcessed { .. } => {}
    });

    // Start FGS
    println!("\nStarting FGS...");
    let (_update, settings) = fgs.process_event(FgsEvent::StartFgs)?;
    monocle::apply_camera_settings(&mut camera, settings)
        .map_err(|e| format!("Failed to apply camera settings: {e:?}"))?;

    // Acquisition phase
    println!("Acquisition phase ({} frames)...", args.acquisition_frames);
    for i in 0..args.acquisition_frames {
        let (frame, metadata) = camera
            .capture_frame()
            .map_err(|e| format!("Camera capture failed: {e}"))?;
        let (_update, settings) = fgs.process_frame(frame.view(), metadata.timestamp)?;
        monocle::apply_camera_settings(&mut camera, settings)
            .map_err(|e| format!("Failed to apply camera settings: {e:?}"))?;
        if args.verbose {
            println!("  Frame {}/{} captured", i + 1, args.acquisition_frames);
        }
    }

    // Calibration frame
    println!("Calibration phase...");
    let (frame, metadata) = camera
        .capture_frame()
        .map_err(|e| format!("Camera capture failed: {e}"))?;
    let (_update, settings) = fgs.process_frame(frame.view(), metadata.timestamp)?;
    monocle::apply_camera_settings(&mut camera, settings)
        .map_err(|e| format!("Failed to apply camera settings: {e:?}"))?;

    // Check if we're tracking
    if !matches!(fgs.state(), FgsState::Tracking { .. }) {
        eprintln!("Warning: FGS did not enter tracking state");
    } else {
        println!("Tracking established!");
    }

    // Tracking phase
    let total_duration = Duration::from_secs_f64(args.duration);
    let frame_interval = Duration::from_millis(frame_interval_ms);
    let num_frames = (total_duration.as_millis() / frame_interval.as_millis()) as usize;

    println!(
        "\nTracking phase ({} frames over {:.1} seconds)...",
        num_frames, args.duration
    );

    // Track start time for converting Instant to Duration

    for frame_num in 0..num_frames {
        // Process next frame
        let (frame, metadata) = camera
            .capture_frame()
            .map_err(|e| format!("Camera capture failed: {e}"))
            .unwrap_or_else(|e| panic!("Frame {frame_num}: {e}"));
        let (update_opt, settings) = fgs
            .process_frame(frame.view(), metadata.timestamp)
            .unwrap_or_else(|e| panic!("Frame {frame_num}: Failed to process frame: {e}"));
        monocle::apply_camera_settings(&mut camera, settings).unwrap_or_else(|e| {
            panic!("Frame {frame_num}: Failed to apply camera settings: {e:?}")
        });
        let update = update_opt.unwrap_or_else(|| {
            panic!("Frame {frame_num}: Should have guidance update during tracking")
        });

        let current_time = update.timestamp.to_duration();

        // Get actual pointing from motion at current time
        let actual_pointing = motion_for_tracking.get_pointing(current_time);

        // Create projector for actual pointing to compute where star should be
        let actual_projector = StarProjector::new(
            &actual_pointing,
            radians_per_pixel,
            sensor_width,
            sensor_height,
        );

        // Project the base (stationary) star position to get actual pixel position
        // The star is at the base_pointing when stationary
        let actual_position = actual_projector
            .project(&base_pointing)
            .expect("Failed to project star position - star may be outside field of view");
        let (actual_x, actual_y) = (actual_position.0, actual_position.1);

        // Get estimated position from FGS
        let has_lock = *lock_established.lock().unwrap();
        let (est_x, est_y) = *estimated_position.lock().unwrap();

        // Add data point to plotter
        plotter.add_point(TrackingDataPoint {
            time: current_time,
            actual_x,
            actual_y,
            estimated_x: if has_lock { est_x } else { actual_x },
            estimated_y: if has_lock { est_y } else { actual_y },
            is_frame_arrival: true,
            has_lock,
        });

        // Progress indicator
        if !args.verbose && frame_num % 10 == 0 {
            print!(".");
            use std::io::Write;
            std::io::stdout().flush()?;
        }
    }

    if !args.verbose {
        println!(); // New line after progress dots
    }

    // Generate the plots
    println!("\nGenerating plots...");
    plotter.generate_plot()?;
    println!("✅ Tracking plot saved to plots/{output_filename}");

    // Generate residuals plot
    plotter.generate_residuals_plot()?;
    let residuals_filename = output_filename.replace(".png", "_residuals.png");
    println!("✅ Residuals plot saved to plots/{residuals_filename}");

    // Stop FGS
    let (_update, settings) = fgs.process_event(FgsEvent::StopFgs)?;
    monocle::apply_camera_settings(&mut camera, settings)
        .map_err(|e| format!("Failed to apply camera settings: {e:?}"))?;
    println!("FGS stopped successfully");

    Ok(())
}
