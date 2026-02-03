//! Capture frames from NSV455 in test pattern mode.
//!
//! This tool enables a sensor test pattern and captures N full-frame images,
//! saving them as 16-bit FITS files. Useful for validating camera pipeline
//! without optical input.

use anyhow::Result;
use clap::{Parser, ValueEnum};
use hardware::nsv455::camera::controls::TestPattern;
use hardware::nsv455::camera::nsv455_camera::NSV455Camera;
use shared::camera_interface::CameraInterface;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TestPatternArg {
    /// No test pattern (normal operation)
    None,
    /// Vertical bar pattern
    Vertical,
    /// Horizontal bar pattern
    Horizontal,
    /// Gradient pattern
    Gradient,
}

impl From<TestPatternArg> for TestPattern {
    fn from(arg: TestPatternArg) -> Self {
        match arg {
            TestPatternArg::None => TestPattern::None,
            TestPatternArg::Vertical => TestPattern::Vertical,
            TestPatternArg::Horizontal => TestPattern::Horizontal,
            TestPatternArg::Gradient => TestPattern::Gradient,
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Capture frames from NSV455 in test pattern mode"
)]
struct Args {
    /// V4L2 device path
    #[arg(short, long, default_value = "/dev/video0")]
    device: String,

    /// Test pattern to use
    #[arg(short, long, value_enum, default_value = "gradient")]
    pattern: TestPatternArg,

    /// Number of frames to capture
    #[arg(short, long, default_value = "10")]
    frames: usize,

    /// Output directory for captured frames
    #[arg(short, long, default_value = ".")]
    output: PathBuf,

    /// Exposure time in milliseconds
    #[arg(short, long, default_value = "100")]
    exposure_ms: u64,

    /// Skip first N frames (allow sensor to stabilize)
    #[arg(long, default_value = "5")]
    skip: usize,
}

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let args = Args::parse();

    println!("NSV455 Test Pattern Capture");
    println!("===========================");
    println!("Device:      {}", args.device);
    println!("Pattern:     {:?}", args.pattern);
    println!("Frames:      {}", args.frames);
    println!("Skip:        {} initial frames", args.skip);
    println!("Exposure:    {} ms", args.exposure_ms);
    println!("Output dir:  {}", args.output.display());
    println!();

    // Create output directory if needed
    std::fs::create_dir_all(&args.output)?;

    // Initialize camera
    println!("Initializing camera...");
    let mut camera = NSV455Camera::from_device(args.device)?;

    // Configure test pattern
    let pattern: TestPattern = args.pattern.into();
    camera.set_test_pattern(pattern);
    println!("Test pattern set to: {:?}", camera.get_test_pattern());

    // Set exposure
    camera.set_exposure(Duration::from_millis(args.exposure_ms))?;
    println!("Exposure set to: {:?}", camera.get_exposure());

    // Capture frames
    let mut captured = 0;
    let mut skipped = 0;

    println!(
        "\nStarting capture ({} frames, skipping first {})...",
        args.frames, args.skip
    );

    camera.stream(&mut |frame, metadata| {
        if skipped < args.skip {
            skipped += 1;
            print!("\rSkipping frame {}/{}...", skipped, args.skip);
            std::io::Write::flush(&mut std::io::stdout()).ok();
            return true;
        }

        captured += 1;
        let frame_num = captured;

        // Generate filename
        let filename = args.output.join(format!(
            "test_pattern_{:?}_{:04}.fits",
            args.pattern, frame_num
        ));

        // Save as FITS
        match save_fits(frame, &filename) {
            Ok(_) => {
                println!(
                    "\rCaptured frame {}/{}: {} ({}x{}, seq={})",
                    captured,
                    args.frames,
                    filename.display(),
                    frame.ncols(),
                    frame.nrows(),
                    metadata.frame_number
                );
            }
            Err(e) => {
                eprintln!("\rError saving frame {frame_num}: {e}");
            }
        }

        captured < args.frames
    })?;

    println!(
        "\nCapture complete! {} frames saved to {}",
        captured,
        args.output.display()
    );

    Ok(())
}

/// Save a frame as a 16-bit FITS file
fn save_fits(frame: &ndarray::Array2<u16>, path: &PathBuf) -> Result<()> {
    use std::io::Write;

    let height = frame.nrows();
    let width = frame.ncols();

    // Create FITS header
    let mut header = String::new();
    header.push_str(&format!(
        "{:<80}",
        "SIMPLE  =                    T / FITS standard"
    ));
    header.push_str(&format!(
        "{:<80}",
        "BITPIX  =                   16 / 16-bit unsigned integers"
    ));
    header.push_str(&format!(
        "{:<80}",
        "NAXIS   =                    2 / 2D image"
    ));
    header.push_str(&format!(
        "{:<80}",
        format!("NAXIS1  = {:>20} / Width in pixels", width)
    ));
    header.push_str(&format!(
        "{:<80}",
        format!("NAXIS2  = {:>20} / Height in pixels", height)
    ));
    header.push_str(&format!(
        "{:<80}",
        "BZERO   =                32768 / Offset for unsigned 16-bit"
    ));
    header.push_str(&format!(
        "{:<80}",
        "BSCALE  =                    1 / Scale factor"
    ));
    header.push_str(&format!("{:<80}", "END"));

    // Pad header to multiple of 2880 bytes
    let header_len = header.len();
    let padding_needed = (2880 - (header_len % 2880)) % 2880;
    header.push_str(&" ".repeat(padding_needed));

    // Open file and write header
    let mut file = std::fs::File::create(path)?;
    file.write_all(header.as_bytes())?;

    // Write pixel data (FITS uses big-endian, signed 16-bit with BZERO offset)
    // Convert u16 to i16 by subtracting 32768
    for row in frame.rows() {
        for &pixel in row {
            let signed_pixel = (pixel as i32 - 32768) as i16;
            file.write_all(&signed_pixel.to_be_bytes())?;
        }
    }

    // Pad data to multiple of 2880 bytes
    let data_len = width * height * 2;
    let data_padding = (2880 - (data_len % 2880)) % 2880;
    file.write_all(&vec![0u8; data_padding])?;

    Ok(())
}
