use anyhow::Result;
use flight_software::v4l2_capture::{CameraConfig, CaptureSession, ResolutionProfile, V4L2Capture};
use std::time::Instant;
use tracing::{error, info};

fn test_resolution_profile(device: &str, profile: &ResolutionProfile) -> Result<()> {
    info!(
        "Testing {} - Resolution {}x{} @ {} Hz",
        device, profile.width, profile.height, profile.framerate
    );

    let config = CameraConfig {
        device_path: device.to_string(),
        width: profile.width,
        height: profile.height,
        framerate: profile.framerate,
        gain: 360,
        exposure: 140,
        black_level: 4095,
    };

    let capture = V4L2Capture::new(config)?;

    let start = Instant::now();

    let frames = capture.capture_frames_with_skip(1, profile.test_frames as usize)?;

    let elapsed = start.elapsed();

    if let Some(frame) = frames.first() {
        let filename = format!("test_{}x{}.raw", profile.width, profile.height);
        std::fs::write(&filename, frame)?;

        let file_size = std::fs::metadata(&filename)?.len();
        info!(
            "Captured frame saved to {} ({} bytes) in {:?}",
            filename, file_size, elapsed
        );
    }

    Ok(())
}

fn test_single_capture(device: &str) -> Result<()> {
    info!("Testing single frame capture from {}", device);

    let config = CameraConfig {
        device_path: device.to_string(),
        ..Default::default()
    };

    let capture = V4L2Capture::new(config)?;

    let start = Instant::now();
    let frame = capture.capture_single_frame()?;
    let elapsed = start.elapsed();

    let filename = "single_capture.raw";
    std::fs::write(filename, &frame)?;

    info!(
        "Single frame captured: {} bytes in {:?}",
        frame.len(),
        elapsed
    );

    Ok(())
}

fn test_continuous_capture(device: &str, count: usize) -> Result<()> {
    info!(
        "Testing continuous capture of {} frames from {}",
        count, device
    );

    let config = CameraConfig {
        device_path: device.to_string(),
        ..Default::default()
    };

    let mut session = CaptureSession::new(&config)?;
    session.start_stream()?;

    let start = Instant::now();

    for i in 0..count {
        let frame_start = Instant::now();
        let frame = session.capture_frame()?;
        let frame_time = frame_start.elapsed();

        info!(
            "Frame {}/{}: {} bytes captured in {:?}",
            i + 1,
            count,
            frame.len(),
            frame_time
        );
    }

    let total_time = start.elapsed();
    let fps = count as f64 / total_time.as_secs_f64();

    info!(
        "Captured {} frames in {:?} ({:.2} fps)",
        count, total_time, fps
    );

    session.stop_stream();
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let device = std::env::var("VIDEO_DEVICE").unwrap_or_else(|_| "/dev/video0".to_string());

    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "single".to_string());

    match mode.as_str() {
        "single" => {
            test_single_capture(&device)?;
        }
        "continuous" => {
            let count = std::env::args()
                .nth(2)
                .and_then(|s| s.parse().ok())
                .unwrap_or(10);
            test_continuous_capture(&device, count)?;
        }
        "profiles" => {
            let profiles = ResolutionProfile::standard_profiles();
            for profile in &profiles {
                if let Err(e) = test_resolution_profile(&device, profile) {
                    error!("Failed to test profile: {}", e);
                }
            }
        }
        "custom" => {
            let width: u32 = std::env::args()
                .nth(2)
                .and_then(|s| s.parse().ok())
                .unwrap_or(1024);
            let height: u32 = std::env::args()
                .nth(3)
                .and_then(|s| s.parse().ok())
                .unwrap_or(1024);
            let framerate: u32 = std::env::args()
                .nth(4)
                .and_then(|s| s.parse().ok())
                .unwrap_or(23_000_000);

            let profile = ResolutionProfile {
                width,
                height,
                framerate,
                test_frames: 10,
            };

            test_resolution_profile(&device, &profile)?;
        }
        _ => {
            eprintln!(
                "Usage: {} [mode] [options]",
                std::env::args().next().unwrap()
            );
            eprintln!("Modes:");
            eprintln!("  single            - Capture single frame");
            eprintln!("  continuous [n]    - Capture n frames (default: 10)");
            eprintln!("  profiles          - Test all standard resolution profiles");
            eprintln!("  custom w h fps    - Test custom resolution");
            eprintln!();
            eprintln!("Environment:");
            eprintln!("  VIDEO_DEVICE      - Device path (default: /dev/video0)");
        }
    }

    Ok(())
}
