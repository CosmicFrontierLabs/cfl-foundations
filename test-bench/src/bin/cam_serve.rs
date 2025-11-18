//! Unified camera HTTP server for all camera types.

use anyhow::Result;
use clap::Parser;
use test_bench::camera_init::{initialize_camera, CameraArgs};
use test_bench::camera_server::CommonServerArgs;
use tracing::info;

#[derive(Parser, Debug)]
#[command(author, version, about = "HTTP server for all camera types")]
struct Args {
    #[command(flatten)]
    camera: CameraArgs,

    #[command(flatten)]
    server: CommonServerArgs,
}

fn check_frontend_files() -> Result<()> {
    let wasm_file = "test-bench-frontend/dist/camera/camera_wasm_bg.wasm";
    let js_file = "test-bench-frontend/dist/camera/camera_wasm.js";

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

    info!("Starting camera server...");
    test_bench::camera_server::run_server(camera, args.server).await
}
