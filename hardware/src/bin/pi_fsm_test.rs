//! Test binary for PI E-727 Fast Steering Mirror communication.
//!
//! Connects to E-727 via TCP, queries device information and axis status.

use anyhow::Result;
use hardware::pi::{DEFAULT_FSM_IP, E727};
use tracing::info;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    info!("Connecting to PI E-727 at {}...", DEFAULT_FSM_IP);
    let mut fsm = E727::connect_ip(DEFAULT_FSM_IP)?;

    info!("Device ID: {}", fsm.idn()?);
    info!("Available axes: {:?}", fsm.axes());

    for axis in fsm.axes() {
        let (min, max) = fsm.get_travel_range(axis)?;
        let unit = fsm.get_unit(axis)?;
        let servo = fsm.get_servo(axis)?;
        let pos = fsm.get_position(axis)?;

        info!("Axis {axis}: range [{min:.3}, {max:.3}] {unit}, servo={servo}, pos={pos:.3}");
    }

    let on_target = fsm.all_on_target()?;
    info!("On target: {:?}", on_target);

    info!("Demo complete!");
    Ok(())
}
