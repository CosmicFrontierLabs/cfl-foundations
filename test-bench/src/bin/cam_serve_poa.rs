//! PlayerOne astronomy camera HTTP server.
//!
//! IMPORTANT: This binary CANNOT be combined with cam_serve_nsv in the same executable.
//! The v4l2 libraries used by NSV455 cameras enumerate and claim USB video devices at
//! program initialization, which conflicts with the PlayerOne SDK's USB device access.
//! This causes PlayerOne cameras to fail initialization with "Camera not found" errors
//! even when the PlayerOne code path is not executed.

use clap::Parser;
use test_bench::camera_server::{run_server, CommonServerArgs};
use test_bench::poa::camera::PlayerOneCamera;
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about = "HTTP server for PlayerOne astronomy cameras")]
struct Args {
    #[command(flatten)]
    common: CommonServerArgs,

    #[arg(short = 'i', long, default_value = "0")]
    camera_id: i32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    info!("Initializing PlayerOne camera with ID {}", args.camera_id);
    let camera = PlayerOneCamera::new(args.camera_id)
        .map_err(|e| anyhow::anyhow!("Failed to initialize POA camera: {e}"))?;

    run_server(camera, args.common).await
}
