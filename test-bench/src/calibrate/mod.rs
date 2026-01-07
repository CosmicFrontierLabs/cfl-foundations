//! Calibration display module.
//!
//! Provides shared infrastructure for displaying calibration patterns,
//! used by both the CLI tool (calibrate) and web server (calibrate_serve).

mod display;
mod pattern;
mod pattern_builders;
mod schema;

pub use display::{run_display, DisplayConfig, DynamicPattern, FixedPattern, PatternSource};
pub use pattern::PatternConfig;
pub use pattern_builders::{create_gyro_walk, create_optical_calibration, load_motion_profile};
pub use schema::{
    get_pattern_schemas, parse_pattern_request, pattern_to_dynamic, ControlSpec, PatternSpec,
    SchemaResponse,
};
