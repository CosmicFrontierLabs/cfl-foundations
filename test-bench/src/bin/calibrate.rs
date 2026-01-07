use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::rect::Rect;
use shared::config_storage::ConfigStorage;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use test_bench::calibrate::{
    create_gyro_walk, create_optical_calibration, load_motion_profile, PatternConfig,
};
use test_bench::display_patterns as patterns;
use test_bench::display_utils::{
    get_display_resolution, list_displays, resolve_display_index, SdlResultExt,
};

#[derive(Debug, Clone, ValueEnum)]
enum PatternType {
    Check,
    Usaf,
    Static,
    Pixel,
    April,
    CirclingPixel,
    Uniform,
    WigglingGaussian,
    PixelGrid,
    SiemensStar,
    MotionProfile,
    GyroWalk,
    OpticalCalibration,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Display calibration utility", long_about = None)]
struct Args {
    #[arg(short, long, help = "Display index to use (0-based)")]
    display: Option<u32>,

    #[arg(short, long, help = "List available displays and exit")]
    list: bool,

    #[arg(
        short,
        long,
        help = "Pattern type to display",
        value_enum,
        default_value = "april"
    )]
    pattern: PatternType,

    #[arg(long, help = "Target width in pixels (defaults to display width)")]
    width: Option<u32>,

    #[arg(long, help = "Target height in pixels (defaults to display height)")]
    height: Option<u32>,

    #[arg(
        long,
        help = "Size of each checker square in pixels",
        default_value = "100"
    )]
    checker_size: u32,

    #[arg(
        long,
        help = "Size of each static pixel block in pixels",
        default_value = "1"
    )]
    static_pixel_size: u32,

    #[arg(
        long,
        help = "Number of orbiting pixels in circling pattern",
        default_value = "1"
    )]
    orbit_count: u32,

    #[arg(
        long,
        help = "Orbit radius as percentage of FOV (0-100) for circling-pixel pattern",
        default_value = "50"
    )]
    orbit_radius_percent: u32,

    #[arg(
        long,
        help = "Grid size for optical-calibration pattern (NxN points)",
        default_value = "5"
    )]
    calibration_grid_size: usize,

    #[arg(
        long,
        help = "Grid spacing in pixels for optical-calibration pattern",
        default_value = "5.0"
    )]
    calibration_grid_spacing: f64,

    #[arg(
        long,
        help = "Spot FWHM in pixels for optical-calibration pattern",
        default_value = "2.0"
    )]
    spot_fwhm_pixels: f64,

    #[arg(
        long,
        help = "Warmup time in seconds to ignore before collecting calibration data",
        default_value = "5.0"
    )]
    warmup_secs: f64,

    #[arg(long, help = "Uniform brightness level (0-255)", default_value = "0")]
    uniform_level: u8,

    #[arg(
        long,
        help = "Gaussian FWHM (Full Width Half Maximum in pixels)",
        default_value = "47"
    )]
    gaussian_fwhm: f64,

    #[arg(long, help = "Wiggle radius in pixels", default_value = "3")]
    wiggle_radius_pixels: f64,

    #[arg(
        long,
        help = "Maximum intensity for wiggling gaussian (0-255)",
        default_value = "255"
    )]
    gaussian_intensity: f64,

    #[arg(long, help = "Pixel grid spacing in pixels", default_value = "50")]
    grid_spacing: u32,

    #[arg(long, help = "Number of spokes in Siemens star", default_value = "24")]
    siemens_spokes: u32,

    #[arg(long, help = "Path to PNG image for motion profile pattern")]
    image_path: Option<PathBuf>,

    #[arg(
        long,
        help = "Path to CSV file with motion profile (t, x, y in seconds, pixels, pixels)"
    )]
    motion_csv: Option<PathBuf>,

    #[arg(
        long,
        help = "Motion scaling percentage (100 = normal, 50 = half motion, 200 = double motion)",
        default_value = "100.0"
    )]
    motion_scale_percent: f64,

    #[arg(
        long,
        help = "Pixel size in micrometers for gyro-walk pattern",
        default_value = "3.45"
    )]
    gyro_pixel_size_um: f64,

    #[arg(
        long,
        help = "Focal length in millimeters for gyro-walk pattern",
        default_value = "50.0"
    )]
    gyro_focal_length_mm: f64,

    #[arg(
        long,
        help = "Target frame rate in Hz for gyro-walk simulation",
        default_value = "60.0"
    )]
    gyro_frame_rate_hz: f64,

    #[arg(
        long,
        help = "Motion multiplier for gyro-walk pattern (1.0 = normal, 2.0 = double motion)",
        default_value = "1.0"
    )]
    gyro_motion_scale: f64,

    #[arg(
        long,
        help = "ZMQ SUB endpoint for closed-loop pattern (e.g., tcp://orin.tail12345.ts.net:5555)"
    )]
    zmq_sub: Option<String>,

    #[arg(short, long, help = "Invert pattern colors (black <-> white)")]
    invert: bool,

    #[arg(short, long, help = "Save pattern to PNG file instead of displaying")]
    output: Option<PathBuf>,
}

/// Convert CLI arguments to PatternConfig and print pattern info.
fn args_to_pattern_config(args: &Args, width: u32, height: u32) -> Result<PatternConfig> {
    match args.pattern {
        PatternType::Check => {
            println!("Generating checkerboard pattern");
            println!("  Checker size: {}px", args.checker_size);
            Ok(PatternConfig::Check {
                checker_size: args.checker_size,
            })
        }
        PatternType::Usaf => {
            println!("Rendering USAF-1951 test target from SVG");
            Ok(PatternConfig::Usaf)
        }
        PatternType::Static => {
            println!("Generating static pattern");
            println!("  Block size: {}px", args.static_pixel_size);
            Ok(PatternConfig::Static {
                pixel_size: args.static_pixel_size,
            })
        }
        PatternType::Pixel => {
            println!("Generating center pixel pattern");
            Ok(PatternConfig::Pixel)
        }
        PatternType::April => {
            println!("Generating AprilTag array pattern");
            Ok(PatternConfig::April)
        }
        PatternType::CirclingPixel => {
            println!("Generating circling pixel pattern");
            println!("  Orbit count: {}", args.orbit_count);
            println!("  Orbit radius: {}% FOV", args.orbit_radius_percent);
            println!("  Rotation period: 60 seconds");
            Ok(PatternConfig::CirclingPixel {
                orbit_count: args.orbit_count,
                orbit_radius_percent: args.orbit_radius_percent,
            })
        }
        PatternType::Uniform => {
            println!("Generating uniform screen");
            println!("  Brightness level: {}", args.uniform_level);
            Ok(PatternConfig::Uniform {
                level: args.uniform_level,
            })
        }
        PatternType::WigglingGaussian => {
            println!("Generating wiggling gaussian pattern");
            println!("  Gaussian FWHM: {} pixels", args.gaussian_fwhm);
            println!("  Wiggle radius: {} pixels", args.wiggle_radius_pixels);
            println!("  Maximum intensity: {}", args.gaussian_intensity);
            println!("  Rotation period: 10 seconds");
            Ok(PatternConfig::WigglingGaussian {
                fwhm: args.gaussian_fwhm,
                wiggle_radius: args.wiggle_radius_pixels,
                intensity: args.gaussian_intensity,
            })
        }
        PatternType::PixelGrid => {
            println!("Generating pixel grid pattern");
            println!("  Grid spacing: {} pixels", args.grid_spacing);
            Ok(PatternConfig::PixelGrid {
                spacing: args.grid_spacing,
            })
        }
        PatternType::SiemensStar => {
            println!("Generating Siemens star pattern");
            println!("  Number of spokes: {}", args.siemens_spokes);
            Ok(PatternConfig::SiemensStar {
                spokes: args.siemens_spokes,
            })
        }
        PatternType::MotionProfile => {
            let image_path = args
                .image_path
                .as_ref()
                .context("--image-path required for motion-profile pattern")?;
            let motion_csv = args
                .motion_csv
                .as_ref()
                .context("--motion-csv required for motion-profile pattern")?;

            println!("Loading motion profile pattern");
            println!("  Image: {}", image_path.display());
            println!("  Motion CSV: {}", motion_csv.display());
            println!("  Motion scale: {}%", args.motion_scale_percent);

            load_motion_profile(
                image_path,
                motion_csv,
                width,
                height,
                args.motion_scale_percent / 100.0,
            )
        }
        PatternType::GyroWalk => {
            let image_path = args
                .image_path
                .as_ref()
                .context("--image-path required for gyro-walk pattern")?;

            println!("Loading gyro-walk pattern");
            println!("  Image: {}", image_path.display());
            println!("  Pixel size: {} um", args.gyro_pixel_size_um);
            println!("  Focal length: {} mm", args.gyro_focal_length_mm);
            println!("  Frame rate: {} Hz", args.gyro_frame_rate_hz);
            println!("  Motion scale: {}", args.gyro_motion_scale);

            create_gyro_walk(
                image_path,
                width,
                height,
                args.gyro_pixel_size_um,
                args.gyro_focal_length_mm,
                args.gyro_motion_scale,
                args.gyro_frame_rate_hz,
            )
        }
        PatternType::OpticalCalibration => {
            let zmq_endpoint = args
                .zmq_sub
                .as_deref()
                .context("--zmq-sub is required for optical-calibration pattern")?;

            let grid_radius =
                args.calibration_grid_spacing * (args.calibration_grid_size - 1) as f64 / 2.0;
            println!("Generating optical calibration pattern");
            println!(
                "  Grid: {}x{} points, {} pixel spacing (radius {:.1} pixels)",
                args.calibration_grid_size,
                args.calibration_grid_size,
                args.calibration_grid_spacing,
                grid_radius
            );
            println!("  Spot FWHM: {} pixels", args.spot_fwhm_pixels);
            println!("  Warmup period: {} seconds", args.warmup_secs);
            println!("  ZMQ SUB endpoint: {zmq_endpoint}");

            create_optical_calibration(
                zmq_endpoint,
                args.calibration_grid_size,
                args.calibration_grid_spacing,
                width,
                height,
                args.spot_fwhm_pixels,
                Duration::from_secs_f64(args.warmup_secs),
            )
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let sdl_context = sdl2::init().map_err(|e| anyhow::anyhow!("SDL init failed: {e}"))?;
    let video_subsystem = sdl_context
        .video()
        .map_err(|e| anyhow::anyhow!("Video subsystem init failed: {e}"))?;

    if args.list {
        return list_displays(&video_subsystem);
    }

    let display_index = resolve_display_index(&video_subsystem, args.display)?;

    let bounds = video_subsystem
        .display_bounds(display_index as i32)
        .sdl_context("Failed to get display bounds")?;
    let mode = video_subsystem
        .desktop_display_mode(display_index as i32)
        .sdl_context("Failed to get display mode")?;

    let (display_width, display_height) = get_display_resolution(&video_subsystem, display_index)?;

    let pattern_width = args.width.unwrap_or(display_width);
    let pattern_height = args.height.unwrap_or(display_height);

    // Convert CLI args to PatternConfig
    let pattern_config = args_to_pattern_config(&args, pattern_width, pattern_height)?;

    println!("  Pattern size: {pattern_width}x{pattern_height}");
    println!(
        "  Display {}: {}x{} at ({}, {})",
        display_index,
        mode.w,
        mode.h,
        bounds.x(),
        bounds.y()
    );

    // Generate initial pattern image
    let mut img = pattern_config.generate(pattern_width, pattern_height)?;

    if args.invert {
        PatternConfig::apply_invert(&mut img);
    }

    // Save to file if requested
    if let Some(output_path) = args.output {
        img.save(&output_path)
            .with_context(|| format!("Failed to save image to {}", output_path.display()))?;
        println!("Pattern saved to {}", output_path.display());
        return Ok(());
    }

    // Set up SDL window
    let window_title = pattern_config.display_name();
    let window = video_subsystem
        .window(window_title, mode.w as u32, mode.h as u32)
        .position(bounds.x(), bounds.y())
        .fullscreen_desktop()
        .build()
        .context("Failed to create window")?;

    let mut canvas = window
        .into_canvas()
        .build()
        .context("Failed to create canvas")?;
    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator
        .create_texture_streaming(
            sdl2::pixels::PixelFormatEnum::RGB24,
            pattern_width,
            pattern_height,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create texture: {e:?}"))?;

    texture
        .update(None, img.as_raw(), (pattern_width * 3) as usize)
        .map_err(|e| anyhow::anyhow!("Failed to update texture: {e:?}"))?;

    // Calculate scaling for non-square displays
    let window_width = mode.w as u32;
    let window_height = mode.h as u32;

    let scale_x = window_width as f32 / pattern_width as f32;
    let scale_y = window_height as f32 / pattern_height as f32;
    let scale = scale_x.min(scale_y);

    let scaled_width = (pattern_width as f32 * scale) as u32;
    let scaled_height = (pattern_height as f32 * scale) as u32;

    let x = (window_width - scaled_width) / 2;
    let y = (window_height - scaled_height) / 2;

    let dst_rect = Rect::new(x as i32, y as i32, scaled_width, scaled_height);

    let mut event_pump = sdl_context
        .event_pump()
        .map_err(|e| anyhow::anyhow!("Failed to get event pump: {e}"))?;

    // Set up animation buffer if needed
    let mut buffer = if pattern_config.is_animated() {
        Some(vec![0u8; (pattern_width * pattern_height * 3) as usize])
    } else {
        None
    };

    let mut frame_count = 0u64;
    let mut last_fps_report = std::time::Instant::now();

    'running: loop {
        frame_count += 1;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape | Keycode::Q),
                    ..
                } => break 'running,
                _ => {}
            }
        }

        // Update animated patterns
        if let Some(ref mut buf) = buffer {
            pattern_config.generate_into_buffer(buf, pattern_width, pattern_height);

            let buffer_to_use: &[u8] = if args.invert {
                // Apply inversion to buffer (need temporary storage)
                buf.iter_mut().for_each(|b| *b = 255 - *b);
                buf
            } else {
                buf
            };

            texture
                .update(None, buffer_to_use, (pattern_width * 3) as usize)
                .map_err(|e| anyhow::anyhow!("Failed to update texture: {e}"))?;

            // Undo inversion so next frame starts fresh
            if args.invert {
                buf.iter_mut().for_each(|b| *b = 255 - *b);
            }
        }

        canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
        canvas.clear();
        canvas
            .copy(&texture, None, Some(dst_rect))
            .map_err(|e| anyhow::anyhow!("Failed to copy texture: {e}"))?;
        canvas.present();

        let elapsed = last_fps_report.elapsed();
        if elapsed.as_secs() >= 1 {
            let fps = frame_count as f64 / elapsed.as_secs_f64();
            println!("FPS: {fps:.1}");
            frame_count = 0;
            last_fps_report = std::time::Instant::now();
        }
    }

    // Save optical calibration on exit
    if let PatternConfig::OpticalCalibration { runner, .. } = &pattern_config {
        save_optical_calibration(runner);
    }

    Ok(())
}

/// Save optical calibration results on exit.
fn save_optical_calibration(runner: &Arc<Mutex<patterns::optical_calibration::CalibrationRunner>>) {
    let state = runner.lock().unwrap();
    if let Some(alignment) = state.estimate_transform() {
        let (sx, sy) = alignment.scale();
        println!(
            "Calibration: scale=({:.4}, {:.4}), rot={:.2} deg, offset=({:.1}, {:.1})",
            sx,
            sy,
            alignment.rotation_degrees(),
            alignment.tx,
            alignment.ty
        );
        println!("  Points used: {}", alignment.num_points);
        if let Some(rms) = alignment.rms_error {
            println!("  RMS error: {rms:.3} pixels");
        }

        match ConfigStorage::new().and_then(|s| s.save_optical_alignment(&alignment)) {
            Ok(path) => println!("Saved to: {}", path.display()),
            Err(e) => eprintln!("Failed to save: {e}"),
        }
    } else {
        println!("No calibration data to save (not enough points collected)");
    }
}
