use anyhow::Result;
use clap::{Parser, ValueEnum};
use flight_software::camera::neutralino_imx455::calculate_stride;
use flight_software::gpio::{GpioController, ORIN_PX04_LINE};
use flight_software::v4l2_capture::{CameraConfig, CaptureSession};
use image::{ImageBuffer, Luma};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::info;

// Barker-13 sequence: [1, 1, 1, 1, 1, -1, -1, 1, 1, -1, 1, -1, 1]
// Represented as bits where true = high, false = low
const BARKER_13: [bool; 13] = [
    true, true, true, true, true, false, false, true, true, false, true, false, true,
];

// Latency measurement timing constants
const LATENCY_SHORT_MODE_ON_US: u64 = 500;
const LATENCY_SHORT_MODE_OFF_US: u64 = 500;
const LATENCY_SHORT_MODE_BITS: usize = 2;

const LATENCY_LONG_MODE_ON_US: u64 = 1000;
const LATENCY_LONG_MODE_OFF_US: u64 = 1000;
const LATENCY_LONG_MODE_BITS: usize = 8;

#[derive(Debug, Clone, ValueEnum)]
enum PatternType {
    Short,
    Long,
    Barker,
}

#[derive(Debug, Clone, ValueEnum)]
enum Resolution {
    #[value(name = "128")]
    R128,
    #[value(name = "256")]
    R256,
    #[value(name = "512")]
    R512,
    #[value(name = "1024")]
    R1024,
    #[value(name = "2048")]
    R2048,
    #[value(name = "4096")]
    R4096,
    #[value(name = "8096x6324")]
    RMax,
}

enum PatternMode {
    Exponential {
        on_us: u64,
        off_us: u64,
        bits: usize,
    },
    Barker {
        symbol_us: u64,
    },
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "/dev/video0")]
    device: String,

    #[arg(short = 'p', long, default_value = "long")]
    pattern: PatternType,

    #[arg(short, long, default_value = "8096x6324")]
    resolution: Resolution,

    #[arg(short, long, default_value_t = 10)]
    frames: usize,

    #[arg(long, default_value = "gpiochip0")]
    gpio_chip: String,

    #[arg(long, default_value_t = ORIN_PX04_LINE)]
    gpio_line: u32,

    #[arg(long, default_value_t = 10000)]
    alignment_delay_us: u64,

    #[arg(short, long)]
    save_last_frame: bool,

    #[arg(short, long)]
    output_name: Option<String>,

    #[arg(long, default_value_t = 3)]
    buffer_count: usize,
}

fn get_uptime_us() -> u64 {
    let uptime = std::fs::read_to_string("/proc/uptime").unwrap_or_else(|_| "0.0 0.0".to_string());

    let uptime_secs: f64 = uptime
        .split_whitespace()
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);

    (uptime_secs * 1_000_000.0) as u64
}

fn get_epoch_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_micros() as u64
}

fn save_frame_as_png(frame_data: &[u8], width: u32, height: u32, filename: &str) -> Result<()> {
    // Convert raw bytes to 16-bit grayscale image, accounting for stride
    let mut img = ImageBuffer::<Luma<u16>, Vec<u16>>::new(width, height);
    let stride = calculate_stride(width, height, frame_data.len());

    // Parse raw data with proper stride handling
    for y in 0..height {
        let row_start = (y as usize) * stride;
        for x in 0..width {
            let pixel_offset = row_start + (x as usize) * 2;
            if pixel_offset + 1 < frame_data.len() {
                let value =
                    u16::from_le_bytes([frame_data[pixel_offset], frame_data[pixel_offset + 1]]);
                img.put_pixel(x, y, Luma([value]));
            }
        }
    }

    // Save as PNG
    img.save(filename)?;
    info!("Saved frame as {}", filename);
    Ok(())
}

fn hot_wait(duration: Duration) {
    let start = Instant::now();
    while start.elapsed() < duration {
        // Spinlock
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    info!(
        "Starting V4L2 latency measurement on {} with {:?} pattern at {:?} resolution",
        args.device, args.pattern, args.resolution
    );

    // Get resolution dimensions
    let (width, height) = match args.resolution {
        Resolution::R128 => (128, 128),
        Resolution::R256 => (256, 256),
        Resolution::R512 => (512, 512),
        Resolution::R1024 => (1024, 1024),
        Resolution::R2048 => (2048, 2048),
        Resolution::R4096 => (4096, 4096),
        Resolution::RMax => (8096, 6324),
    };

    // Get pattern mode
    let pattern_mode = match args.pattern {
        PatternType::Short => PatternMode::Exponential {
            on_us: LATENCY_SHORT_MODE_ON_US,
            off_us: LATENCY_SHORT_MODE_OFF_US,
            bits: LATENCY_SHORT_MODE_BITS,
        },
        PatternType::Long => PatternMode::Exponential {
            on_us: LATENCY_LONG_MODE_ON_US,
            off_us: LATENCY_LONG_MODE_OFF_US,
            bits: LATENCY_LONG_MODE_BITS,
        },
        PatternType::Barker => PatternMode::Barker {
            symbol_us: 1000, // 1ms per symbol as requested
        },
    };

    let mut gpio = GpioController::new(&args.gpio_chip, args.gpio_line)?;
    gpio.request_output("v4l2_latency", 0)?;
    info!(
        "GPIO initialized on {}/{} (PX.04 UART2 TX)",
        args.gpio_chip, args.gpio_line
    );

    let config = CameraConfig {
        device_path: args.device.clone(),
        width,
        height,
        framerate: 23_000_000,
        gain: 360,
        exposure: 140,
        black_level: 4095,
    };

    info!(
        "Camera config: {}x{} @ {} Hz",
        config.width, config.height, config.framerate
    );

    let mut session = CaptureSession::new(&config)?;

    session.start_stream()?;
    info!("Streaming started");

    let t0 = get_uptime_us();
    info!("Image #,  Elapsed Time (usec)");

    for i in 0..args.frames {
        let t1 = get_uptime_us();

        let (_frame, meta) = session.capture_frame()?;
        let start_instance = Instant::now();
        let frame_timestamp_us = meta.timestamp.sec as u64 * 1_000_000 + meta.timestamp.usec as u64;
        info!("Frame {} timestamp: {} us", i, frame_timestamp_us);
        info!(
            "Time Elapsed since start: {} us",
            start_instance.elapsed().as_micros()
        );
        hot_wait(Duration::from_micros(args.alignment_delay_us));

        // Execute the pattern based on mode
        match &pattern_mode {
            PatternMode::Exponential {
                on_us,
                off_us,
                bits,
            } => {
                // Exponential pattern (original blinky)
                for bit in 0..*bits {
                    gpio.set_value(1)?;
                    hot_wait(Duration::from_micros(*on_us));

                    gpio.set_value(0)?;
                    let off_duration = (1 << bit) * off_us;
                    hot_wait(Duration::from_micros(off_duration));
                }
            }
            PatternMode::Barker { symbol_us } => {
                // Barker-13 pattern
                for &symbol in BARKER_13.iter() {
                    gpio.set_value(if symbol { 1 } else { 0 })?;
                    hot_wait(Duration::from_micros(*symbol_us));
                }
                // Return to low after pattern
                gpio.set_value(0)?;
            }
        }
        info!("{} {}", i, t1 - t0);
    }

    let (final_frame, final_meta) = session.capture_frame()?;

    let timestamp_us =
        final_meta.timestamp.sec as u64 * 1_000_000 + final_meta.timestamp.usec as u64;

    let current_uptime = get_uptime_us();
    let current_epoch = get_epoch_us();

    info!("the frame's timestamp in us is: {}", timestamp_us);
    info!("the current uptime in us is: {}", current_uptime);
    info!("the current epochtime in us is: {}", current_epoch);

    // Report nominal timing after pattern completes
    match &pattern_mode {
        PatternMode::Exponential {
            on_us,
            off_us,
            bits,
        } => {
            info!("Exponential pattern ({} bits) nominal timings:", bits);
            for bit in 0..*bits {
                let off_duration = (1 << bit) * off_us;
                info!("  Bit {}: ON {} us, OFF {} us", bit, on_us, off_duration);
            }
            let total_on_time = *bits as u64 * on_us;
            let total_off_time: u64 = (0..*bits).map(|b| (1u64 << b) * off_us).sum();
            info!(
                "  Total: ON {} us, OFF {} us",
                total_on_time, total_off_time
            );
        }
        PatternMode::Barker { symbol_us } => {
            info!("Barker-13 pattern nominal timing:");
            info!("  Symbol duration: {} us", symbol_us);
            info!("  Pattern length: {} symbols", BARKER_13.len());
            info!(
                "  Total duration: {} us",
                symbol_us * BARKER_13.len() as u64
            );
            info!("  Pattern: {:?}", BARKER_13);
        }
    }

    if args.save_last_frame {
        let png_filename = match args.output_name {
            Some(name) => {
                if name.ends_with(".png") {
                    name
                } else {
                    format!("{name}.png")
                }
            }
            None => format!("output_{timestamp_us}_{current_uptime}_{current_epoch}.png"),
        };

        save_frame_as_png(&final_frame, width, height, &png_filename)?;
    }

    session.stop_stream();
    info!("Streaming stopped");

    Ok(())
}
