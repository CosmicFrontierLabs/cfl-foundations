//! Exail Asterix NS Gyro protocol parsing
//!
//! This module provides structures and parsing for raw data packets
//! from the Exail Asterix NS inertial measurement unit.

pub mod angle;
pub mod checksum;
pub mod health_status;
pub mod messages;
pub mod parser;
pub mod temperature;
pub mod time;

pub use angle::{AngleData, ARCSECONDS_PER_LSB};
pub use checksum::{compute_checksum, verify_checksum};
pub use health_status::HealthStatus;
pub use messages::{
    frame_id, FilteredGyroInertialData, FullGyroData, GyroData, RawGyroInertialData, FRAME_ID_MASK,
};
pub use parser::{parse, GyroMessage, ParseError};
pub use temperature::{
    TempDecoder, TemperatureReading, TemperatureSensor, BOARD_TEMP, SIA_FIL_TEMP,
};
pub use time::{GyroTime, TimeInterpretation};
