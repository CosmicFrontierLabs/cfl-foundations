//! Raw GCS protocol test - minimal queries to debug communication

use anyhow::Result;
use hardware::pi::GcsDevice;
use tracing::info;

const PI_IP: &str = "192.168.15.210";

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Connecting to PI device at {}...", PI_IP);
    let mut gcs = GcsDevice::connect_default_port(PI_IP)?;

    info!("Sending *IDN? query...");
    let response = gcs.query("*IDN?")?;
    info!("Got: {}", response.trim());

    info!("Sending SAI? query...");
    let response = gcs.query("SAI?")?;
    info!("Axes: {:?}", response.lines().collect::<Vec<_>>());

    info!("Sending POS? query...");
    let response = gcs.query("POS?")?;
    info!("Positions: {:?}", response.trim());

    info!("Sending SVO? query...");
    let response = gcs.query("SVO?")?;
    info!("Servo states: {:?}", response.trim());

    info!("Done!");
    Ok(())
}
