//! Message types for Exail Asterix NS gyro

mod gyro_data_filt;
mod gyro_data_full;
mod gyro_data_raw;

pub use gyro_data_filt::FilteredGyroInertialData;
pub use gyro_data_full::FullGyroData;
pub use gyro_data_raw::RawGyroInertialData;

/// Mask for extracting the 5-bit frame ID from the message_id byte
pub const FRAME_ID_MASK: u8 = 0x1F;

/// Frame ID constants (5-bit values)
pub mod frame_id {
    pub const RAW_GYRO_BASE: u8 = 17;
    pub const RAW_GYRO: u8 = 18;
    pub const FILTERED_GYRO_BASE: u8 = 19;
    pub const FILTERED_GYRO: u8 = 20;
    pub const FULL_GYRO_BASE: u8 = 21;
    pub const FULL_GYRO: u8 = 22;
}

/// Unified trait for all Exail Asterix NS gyro message types
///
/// Provides access to:
/// - Message metadata (message_id, frame_id, time)
/// - Temperature sensor readings (decoded to Celsius)
/// - Angle measurements (raw and/or filtered, decoded to arcseconds)
///
/// Different message types have different data available:
/// - Raw: raw angles, SIA Filter temp only
/// - Filtered: filtered angles, SIA Filter temp only
/// - Full: both raw and filtered angles, all four temp sensors
pub trait GyroData {
    // Message ID and time methods

    /// Access the raw message_id byte
    fn message_id(&self) -> u8;

    /// Access the gyro_time field
    fn gyro_time(&self) -> &crate::time::GyroTime;

    /// Get the 5-bit frame ID (message type identifier)
    fn frame_id(&self) -> u8 {
        self.message_id() & FRAME_ID_MASK
    }

    /// Check if this is a BASE variant message.
    ///
    /// BASE variants use (GyroTimeTag, TimeBase) time interpretation.
    /// Regular variants use single TimeTag interpretation.
    fn is_base_variant(&self) -> bool {
        matches!(
            self.frame_id(),
            frame_id::RAW_GYRO_BASE | frame_id::FILTERED_GYRO_BASE | frame_id::FULL_GYRO_BASE
        )
    }

    /// Get the time field interpreted according to the message variant.
    ///
    /// Returns:
    /// - `TimeInterpretation::TagAndBase` for BASE variants (17, 19, 21)
    /// - `TimeInterpretation::SingleTag` for regular variants (18, 20, 22)
    fn time_interpretation(&self) -> crate::time::TimeInterpretation {
        self.gyro_time().interpret(self.is_base_variant())
    }

    // Temperature methods

    /// Get all temperature readings from this message as a Vec
    ///
    /// Each reading includes sensor location, raw value, and decoded Celsius temperature
    fn temperature_readings(&self) -> Vec<crate::temperature::TemperatureReading>;

    // Angle data methods

    /// Get raw angle data if available (converted to arcseconds)
    fn raw_angle_data(&self) -> Option<crate::angle::AngleData>;

    /// Get filtered angle data if available (converted to arcseconds)
    fn filtered_angle_data(&self) -> Option<crate::angle::AngleData>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_id_masking() {
        let raw = RawGyroInertialData {
            start_word: 0,
            message_id: 0x80 | frame_id::RAW_GYRO_BASE,
            gyro_time: crate::time::GyroTime::from_bytes([0; 4]),
            angle_x: 0,
            angle_y: 0,
            angle_z: 0,
            sia_fil_temp: 0,
            health_status: crate::health_status::HealthStatus::empty(),
            checksum: 0,
        };
        assert_eq!(raw.frame_id(), frame_id::RAW_GYRO_BASE);
    }

    #[test]
    fn test_is_base_variant() {
        let raw_base = RawGyroInertialData {
            start_word: 0,
            message_id: frame_id::RAW_GYRO_BASE,
            gyro_time: crate::time::GyroTime::from_bytes([0; 4]),
            angle_x: 0,
            angle_y: 0,
            angle_z: 0,
            sia_fil_temp: 0,
            health_status: crate::health_status::HealthStatus::empty(),
            checksum: 0,
        };
        assert!(raw_base.is_base_variant());

        let raw_regular = RawGyroInertialData {
            start_word: 0,
            message_id: frame_id::RAW_GYRO,
            gyro_time: crate::time::GyroTime::from_bytes([0; 4]),
            angle_x: 0,
            angle_y: 0,
            angle_z: 0,
            sia_fil_temp: 0,
            health_status: crate::health_status::HealthStatus::empty(),
            checksum: 0,
        };
        assert!(!raw_regular.is_base_variant());
    }

    #[test]
    fn test_all_message_types_implement_trait() {
        let raw = RawGyroInertialData {
            start_word: 0,
            message_id: frame_id::RAW_GYRO,
            gyro_time: crate::time::GyroTime::from_bytes([0; 4]),
            angle_x: 0,
            angle_y: 0,
            angle_z: 0,
            sia_fil_temp: 0,
            health_status: crate::health_status::HealthStatus::empty(),
            checksum: 0,
        };

        let filtered = FilteredGyroInertialData {
            start_word: 0,
            message_id: frame_id::FILTERED_GYRO,
            gyro_time: crate::time::GyroTime::from_bytes([0; 4]),
            angle_x: 0,
            angle_y: 0,
            angle_z: 0,
            sia_fil_temp: 0,
            health_status: crate::health_status::HealthStatus::empty(),
            checksum: 0,
        };

        let full = FullGyroData {
            start_word: 0,
            message_id: frame_id::FULL_GYRO,
            gyro_time: crate::time::GyroTime::from_bytes([0; 4]),
            raw_ang_x: 0,
            raw_ang_y: 0,
            raw_ang_z: 0,
            fil_ang_x: 0,
            fil_ang_y: 0,
            fil_ang_z: 0,
            so_in_cur: 0,
            cur_com: 0,
            pow_meas_x: 0,
            pow_meas_y: 0,
            pow_meas_z: 0,
            vpi_x: 0,
            vpi_y: 0,
            vpi_z: 0,
            ramp_x: 0,
            ramp_y: 0,
            ramp_z: 0,
            board_temp: 0,
            sia_fil_temp: 0,
            org_fil_temp: 0,
            inter_temp: 0,
            health_status: crate::health_status::HealthStatus::empty(),
            checksum: 0,
        };

        assert_eq!(raw.frame_id(), frame_id::RAW_GYRO);
        assert_eq!(filtered.frame_id(), frame_id::FILTERED_GYRO);
        assert_eq!(full.frame_id(), frame_id::FULL_GYRO);
    }

    #[test]
    fn test_time_interpretation_base_variant() {
        let raw_base = RawGyroInertialData {
            start_word: 0,
            message_id: frame_id::RAW_GYRO_BASE,
            gyro_time: crate::time::GyroTime::from_bytes([0x12, 0x34, 0x56, 0x78]),
            angle_x: 0,
            angle_y: 0,
            angle_z: 0,
            sia_fil_temp: 0,
            health_status: crate::health_status::HealthStatus::empty(),
            checksum: 0,
        };

        let interp = raw_base.time_interpretation();
        match interp {
            crate::time::TimeInterpretation::TagAndBase {
                gyro_time_tag,
                time_base,
            } => {
                assert_eq!(gyro_time_tag, 0x3412);
                assert_eq!(time_base, 0x7856);
            }
            _ => panic!("Expected TagAndBase for BASE variant"),
        }
    }

    #[test]
    fn test_time_interpretation_regular_variant() {
        let raw_regular = RawGyroInertialData {
            start_word: 0,
            message_id: frame_id::RAW_GYRO,
            gyro_time: crate::time::GyroTime::from_bytes([0x12, 0x34, 0x56, 0x78]),
            angle_x: 0,
            angle_y: 0,
            angle_z: 0,
            sia_fil_temp: 0,
            health_status: crate::health_status::HealthStatus::empty(),
            checksum: 0,
        };

        let interp = raw_regular.time_interpretation();
        match interp {
            crate::time::TimeInterpretation::SingleTag(tag) => {
                assert_eq!(tag, 0x78563412);
            }
            _ => panic!("Expected SingleTag for regular variant"),
        }
    }

    #[test]
    fn test_time_interpretation_all_base_variants() {
        let time_bytes = [0xAA, 0xBB, 0xCC, 0xDD];

        let raw = RawGyroInertialData {
            start_word: 0,
            message_id: frame_id::RAW_GYRO_BASE,
            gyro_time: crate::time::GyroTime::from_bytes(time_bytes),
            angle_x: 0,
            angle_y: 0,
            angle_z: 0,
            sia_fil_temp: 0,
            health_status: crate::health_status::HealthStatus::empty(),
            checksum: 0,
        };

        let filtered = FilteredGyroInertialData {
            start_word: 0,
            message_id: frame_id::FILTERED_GYRO_BASE,
            gyro_time: crate::time::GyroTime::from_bytes(time_bytes),
            angle_x: 0,
            angle_y: 0,
            angle_z: 0,
            sia_fil_temp: 0,
            health_status: crate::health_status::HealthStatus::empty(),
            checksum: 0,
        };

        let full = FullGyroData {
            start_word: 0,
            message_id: frame_id::FULL_GYRO_BASE,
            gyro_time: crate::time::GyroTime::from_bytes(time_bytes),
            raw_ang_x: 0,
            raw_ang_y: 0,
            raw_ang_z: 0,
            fil_ang_x: 0,
            fil_ang_y: 0,
            fil_ang_z: 0,
            so_in_cur: 0,
            cur_com: 0,
            pow_meas_x: 0,
            pow_meas_y: 0,
            pow_meas_z: 0,
            vpi_x: 0,
            vpi_y: 0,
            vpi_z: 0,
            ramp_x: 0,
            ramp_y: 0,
            ramp_z: 0,
            board_temp: 0,
            sia_fil_temp: 0,
            org_fil_temp: 0,
            inter_temp: 0,
            health_status: crate::health_status::HealthStatus::empty(),
            checksum: 0,
        };

        assert!(matches!(
            raw.time_interpretation(),
            crate::time::TimeInterpretation::TagAndBase { .. }
        ));
        assert!(matches!(
            filtered.time_interpretation(),
            crate::time::TimeInterpretation::TagAndBase { .. }
        ));
        assert!(matches!(
            full.time_interpretation(),
            crate::time::TimeInterpretation::TagAndBase { .. }
        ));
    }

    #[test]
    fn test_temperature_readings_raw() {
        let raw = RawGyroInertialData {
            start_word: 0,
            message_id: frame_id::RAW_GYRO,
            gyro_time: crate::time::GyroTime::from_bytes([0; 4]),
            angle_x: 0,
            angle_y: 0,
            angle_z: 0,
            sia_fil_temp: 16384,
            health_status: crate::health_status::HealthStatus::empty(),
            checksum: 0,
        };

        let readings = raw.temperature_readings();
        assert_eq!(readings.len(), 1);
        assert_eq!(
            readings[0].sensor,
            crate::temperature::TemperatureSensor::SiaFilter
        );
        assert_eq!(readings[0].raw, 16384);
        assert!(readings[0].celsius.is_some());
    }

    #[test]
    fn test_temperature_readings_filtered() {
        let filtered = FilteredGyroInertialData {
            start_word: 0,
            message_id: frame_id::FILTERED_GYRO,
            gyro_time: crate::time::GyroTime::from_bytes([0; 4]),
            angle_x: 0,
            angle_y: 0,
            angle_z: 0,
            sia_fil_temp: 16384,
            health_status: crate::health_status::HealthStatus::empty(),
            checksum: 0,
        };

        let readings = filtered.temperature_readings();
        assert_eq!(readings.len(), 1);
        assert_eq!(
            readings[0].sensor,
            crate::temperature::TemperatureSensor::SiaFilter
        );
    }

    #[test]
    fn test_temperature_readings_full() {
        let full = FullGyroData {
            start_word: 0,
            message_id: frame_id::FULL_GYRO,
            gyro_time: crate::time::GyroTime::from_bytes([0; 4]),
            raw_ang_x: 0,
            raw_ang_y: 0,
            raw_ang_z: 0,
            fil_ang_x: 0,
            fil_ang_y: 0,
            fil_ang_z: 0,
            so_in_cur: 0,
            cur_com: 0,
            pow_meas_x: 0,
            pow_meas_y: 0,
            pow_meas_z: 0,
            vpi_x: 0,
            vpi_y: 0,
            vpi_z: 0,
            ramp_x: 0,
            ramp_y: 0,
            ramp_z: 0,
            board_temp: 16384,
            sia_fil_temp: 16384,
            org_fil_temp: 16384,
            inter_temp: 16384,
            health_status: crate::health_status::HealthStatus::empty(),
            checksum: 0,
        };

        let readings = full.temperature_readings();
        assert_eq!(readings.len(), 4);

        assert_eq!(
            readings[0].sensor,
            crate::temperature::TemperatureSensor::Board
        );
        assert_eq!(
            readings[1].sensor,
            crate::temperature::TemperatureSensor::SiaFilter
        );
        assert_eq!(
            readings[2].sensor,
            crate::temperature::TemperatureSensor::Organizer
        );
        assert_eq!(
            readings[3].sensor,
            crate::temperature::TemperatureSensor::Interface
        );

        for reading in &readings {
            assert!(reading.celsius.is_some());
            let temp = reading.celsius.unwrap();
            assert!(temp > -50.0 && temp < 100.0);
        }
    }
}
