//! Calibration display module.
//!
//! Provides infrastructure for displaying calibration patterns via the web server.

mod display;
mod pattern;
mod schema;

#[cfg(feature = "ftdi")]
pub mod gyro_emitter;

pub use display::{run_display, DisplayConfig, DynamicPattern, OledSafetyWatchdog, PatternSource};
#[cfg(feature = "ftdi")]
pub use gyro_emitter::{
    list_ftdi_devices_info, spawn_gyro_emitter, GyroEmissionParams, GyroEmitterConfig,
    GyroEmitterHandle, PositionSource,
};
pub use pattern::PatternConfig;
pub use schema::{
    get_pattern_schemas, parse_pattern_request, pattern_to_dynamic, ControlSpec, PatternSpec,
    SchemaResponse,
};
