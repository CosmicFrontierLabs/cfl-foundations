//! Hardware debugging tool for horizontal line artifacts in camera data.
//!
//! This tool captures frames and analyzes them for horizontal line artifacts
//! that may indicate sensor readout issues, timing problems, or interference.
//!
//! Outputs a JSON file with per-row mean and variance statistics for each frame,
//! suitable for analysis in Python.

use anyhow::Result;
use clap::Parser;
use ndarray::Array2;
use serde::Serialize;
use shared::camera_interface::CameraInterface;
use std::path::PathBuf;
use test_bench::camera_init::{initialize_camera, CameraArgs, ExposureArgs};
use tracing::info;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Debug tool for horizontal line artifacts in camera images",
    long_about = "Captures frames from the camera and analyzes them for horizontal line \
        artifacts. These artifacts often appear as bright or dark horizontal bands \
        across the image and can indicate:\n  \
        - Sensor readout timing issues\n  \
        - Electromagnetic interference\n  \
        - Power supply noise\n  \
        - Ground loop problems\n\n\
        The tool provides row-by-row statistics to help identify and characterize \
        these artifacts. Output is JSON for Python analysis."
)]
struct Args {
    #[command(flatten)]
    camera: CameraArgs,

    #[command(flatten)]
    exposure: ExposureArgs,

    #[arg(
        short = 'g',
        long,
        default_value = "100.0",
        help = "Camera gain setting",
        long_help = "Analog gain setting for the camera sensor. Higher gain amplifies \
            both signal and noise, which may make artifacts more visible."
    )]
    gain: f64,

    #[arg(
        short = 'n',
        long,
        default_value = "10",
        help = "Number of frames to capture for analysis",
        long_help = "Number of frames to capture and analyze. More frames provide \
            better statistics for identifying intermittent artifacts."
    )]
    num_frames: usize,

    #[arg(
        short = 'o',
        long,
        default_value = "line_artifact_stats.json",
        help = "Output file path for row statistics JSON",
        long_help = "Path to write the JSON output file containing per-row statistics \
            for each captured frame. The file can be loaded in Python with json.load()."
    )]
    output: PathBuf,
}

#[derive(Serialize)]
struct RowStats {
    mean: f64,
    variance: f64,
    min: u16,
    max: u16,
    median: u16,
}

#[derive(Serialize)]
struct FrameStats {
    frame_number: u64,
    timestamp_sec: u64,
    timestamp_nanos: u64,
    rows: Vec<RowStats>,
}

#[derive(Serialize)]
struct CaptureSession {
    camera_name: String,
    width: usize,
    height: usize,
    exposure_ms: u64,
    gain: f64,
    frames: Vec<FrameStats>,
}

fn compute_row_stats(frame: &Array2<u16>) -> Vec<RowStats> {
    let height = frame.nrows();
    let width = frame.ncols();

    (0..height)
        .map(|row_idx| {
            let row = frame.row(row_idx);
            let n = width as f64;

            let mut values: Vec<u16> = row.iter().copied().collect();
            values.sort_unstable();

            let min = values[0];
            let max = values[values.len() - 1];
            let median = values[values.len() / 2];

            let sum: f64 = row.iter().map(|&v| v as f64).sum();
            let mean = sum / n;

            let variance: f64 = row
                .iter()
                .map(|&v| {
                    let diff = v as f64 - mean;
                    diff * diff
                })
                .sum::<f64>()
                / n;

            RowStats {
                mean,
                variance,
                min,
                max,
                median,
            }
        })
        .collect()
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!("Initializing camera...");
    let mut camera = initialize_camera(&args.camera)?;

    let exposure = args.exposure.as_duration();
    camera
        .set_exposure(exposure)
        .map_err(|e| anyhow::anyhow!("Failed to set exposure: {e}"))?;
    info!("Set exposure to {}ms", args.exposure.exposure_ms);

    camera
        .set_gain(args.gain)
        .map_err(|e| anyhow::anyhow!("Failed to set gain: {e}"))?;
    info!("Set gain to {}", args.gain);

    let geometry = camera.geometry();
    let camera_name = camera.name().to_string();
    info!(
        "Camera: {} ({}x{})",
        camera_name,
        geometry.width(),
        geometry.height()
    );

    info!("Capturing {} frames for analysis...", args.num_frames);

    let mut session = CaptureSession {
        camera_name,
        width: geometry.width(),
        height: geometry.height(),
        exposure_ms: args.exposure.exposure_ms,
        gain: args.gain,
        frames: Vec::with_capacity(args.num_frames),
    };

    let target_frames = args.num_frames;
    let mut frames_captured = 0;

    camera.stream(&mut |frame, metadata| {
        let row_stats = compute_row_stats(frame);

        let frame_stats = FrameStats {
            frame_number: metadata.frame_number,
            timestamp_sec: metadata.timestamp.seconds,
            timestamp_nanos: metadata.timestamp.nanos,
            rows: row_stats,
        };

        session.frames.push(frame_stats);
        frames_captured += 1;

        info!(
            "Captured frame {} ({}/{})",
            metadata.frame_number, frames_captured, target_frames
        );

        frames_captured < target_frames
    })?;

    info!("Writing statistics to {}...", args.output.display());
    let json = serde_json::to_string_pretty(&session)?;
    std::fs::write(&args.output, json)?;

    info!(
        "Done! Captured {} frames, output written to {}",
        session.frames.len(),
        args.output.display()
    );

    Ok(())
}
