use clap::Parser;
use shared::camera_interface::mock::MockCameraInterface;
use shared::camera_interface::CameraConfig;
use std::time::Duration;
use test_bench::camera_server::{run_server, CommonServerArgs};
use test_bench::display_patterns::apriltag;
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about = "HTTP server for mock astronomy camera")]
struct Args {
    #[command(flatten)]
    common: CommonServerArgs,

    #[arg(long, default_value = "1024")]
    width: usize,

    #[arg(long, default_value = "1024")]
    height: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!("Generating AprilTag calibration pattern...");
    info!("  Target size: {}x{}", args.width, args.height);

    let apriltag_frame = apriltag::generate_as_array(args.width, args.height)?;
    let inverted_frame = apriltag_frame.mapv(|v| 65535 - v);

    let config = CameraConfig {
        width: args.width,
        height: args.height,
        exposure: Duration::from_millis(100),
        bit_depth: 16,
    };

    let camera = MockCameraInterface::new_repeating(config, inverted_frame);

    run_server(camera, args.common).await
}
