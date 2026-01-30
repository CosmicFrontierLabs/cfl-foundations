mod e727;
mod gcs;
mod s330;

pub use e727::{
    Axis, PiErrorCode, RecordChannel, RecordTrigger, SpaParam, E727, RECORDER_SAMPLE_RATE_HZ,
};
pub use gcs::{GcsDevice, GcsError, GcsResult, DEFAULT_FSM_IP, DEFAULT_PORT};
pub use s330::{FsmArgs, S330};
