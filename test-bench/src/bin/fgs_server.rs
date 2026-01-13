//! Unified camera server with tracking support.
//!
//! Combines the functionality of cam_serve (web UI, image streaming) and
//! cam_track (monocle FGS tracking) into a single binary. When tracking
//! is enabled via the web UI, the system detects spots, locks on, and tracks them.

use anyhow::{Context, Result};
use clap::Parser;
use shared::config_storage::ConfigStorage;
use test_bench::camera_init::{initialize_camera, CameraArgs};
use test_bench::camera_server::{CommonServerArgs, TrackingConfig};
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about = "Unified camera server with tracking support")]
struct Args {
    #[command(flatten)]
    camera: CameraArgs,

    #[command(flatten)]
    server: CommonServerArgs,

    #[arg(long, default_value = "5")]
    acquisition_frames: usize,

    #[arg(long, default_value = "128")]
    roi_size: usize,

    #[arg(long, default_value = "5.0")]
    detection_threshold_sigma: f64,

    #[arg(long, default_value = "10.0")]
    snr_min: f64,

    #[arg(
        long,
        default_value = "3.0",
        help = "SNR threshold below which tracking is lost"
    )]
    snr_dropout_threshold: f64,

    #[arg(
        long,
        default_value = "7.0",
        help = "Expected FWHM of stars in pixels (used for centroiding and SNR calculation)"
    )]
    fwhm: f64,

    #[arg(
        long,
        help = "ZMQ PUB socket bind address for tracking updates (e.g., tcp://*:5555)"
    )]
    zmq_pub: Option<String>,
}

fn check_frontend_files() -> Result<()> {
    let wasm_file = "test-bench-frontend/dist/fgs/fgs_wasm_bg.wasm";
    let js_file = "test-bench-frontend/dist/fgs/fgs_wasm.js";

    if !std::path::Path::new(wasm_file).exists() || !std::path::Path::new(js_file).exists() {
        anyhow::bail!(
            "Frontend WASM files not found!\n\n\
            The camera server requires compiled Yew frontend files.\n\n\
            To build the frontends, run:\n\
            \x20   ./scripts/build-yew-frontends.sh\n\n\
            Or if you don't have trunk installed:\n\
            \x20   cargo install --locked trunk\n\
            \x20   ./scripts/build-yew-frontends.sh\n\n\
            Missing files:\n\
            \x20   - {wasm_file}\n\
            \x20   - {js_file}"
        );
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    check_frontend_files()?;

    info!("Initializing camera...");
    let camera = initialize_camera(&args.camera)?;

    let config_store = ConfigStorage::new().context("Failed to initialize config storage")?;

    info!(
        "Loading bad pixel map for {} (serial: {})",
        camera.name(),
        camera.get_serial()
    );

    let bad_pixel_map = match config_store.get_bad_pixel_map(camera.name(), &camera.get_serial()) {
        Some(Ok(map)) => {
            info!(
                "Loaded bad pixel map with {} bad pixels",
                map.num_bad_pixels()
            );
            map
        }
        Some(Err(e)) => {
            tracing::warn!(
                "Failed to load bad pixel map for camera {} (serial: {}): {}, using empty map",
                camera.name(),
                camera.get_serial(),
                e
            );
            shared::bad_pixel_map::BadPixelMap::empty()
        }
        None => {
            tracing::warn!(
                "No bad pixel map found for camera {} (serial: {}), using empty map",
                camera.name(),
                camera.get_serial()
            );
            shared::bad_pixel_map::BadPixelMap::empty()
        }
    };

    // Get ROI alignment constraints from camera
    let (roi_h_alignment, roi_v_alignment) = camera.get_roi_offset_alignment();
    info!(
        "Camera ROI alignment constraints: h={}, v={}",
        roi_h_alignment, roi_v_alignment
    );

    let tracking_config = TrackingConfig {
        acquisition_frames: args.acquisition_frames,
        roi_size: args.roi_size,
        detection_threshold_sigma: args.detection_threshold_sigma,
        snr_min: args.snr_min,
        snr_dropout_threshold: args.snr_dropout_threshold,
        fwhm: args.fwhm,
        bad_pixel_map,
        saturation_value: camera.saturation_value(),
        roi_h_alignment,
        roi_v_alignment,
        zmq_pub: args.zmq_pub,
    };

    info!("Starting unified camera server with tracking support...");
    test_bench::camera_server::run_server_with_tracking(camera, args.server, tracking_config).await
}
