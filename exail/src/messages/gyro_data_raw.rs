//! Raw Gyro Inertial Data message

use bytemuck::{Pod, Zeroable};

use super::GyroData;
use crate::angle::AngleData;
use crate::health_status::HealthStatus;
use crate::temperature::{TemperatureReading, TemperatureSensor};
use crate::time::GyroTime;

/// Raw Gyro Inertial Data packet from Exail Asterix NS
///
/// Total packet size: 26 bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C, packed)]
pub struct RawGyroInertialData {
    /// Address of Remote Terminal (1 byte)
    pub start_word: u8,

    /// Message ID for RAW GYRO INERTIAL DATA BASE (1 byte)
    pub message_id: u8,

    /// Time field (4 bytes) - can be interpreted as TimeTag (u32) or (GyroTimeTag, TimeBase) pair
    pub gyro_time: GyroTime,

    /// Measure of angle, X axis (4 bytes)
    pub angle_x: u32,

    /// Measure of angle, Y axis (4 bytes)
    pub angle_y: u32,

    /// Measure of angle, Z axis (4 bytes)
    pub angle_z: u32,

    /// SIA Filter Temperature (2 bytes)
    pub sia_fil_temp: u16,

    /// Health status register (4 bytes)
    pub health_status: HealthStatus,

    /// CRC checksum (2 bytes)
    pub checksum: u16,
}

impl RawGyroInertialData {
    /// Expected packet size in bytes
    pub const PACKET_SIZE: usize = 26;
}

impl GyroData for RawGyroInertialData {
    fn message_id(&self) -> u8 {
        self.message_id
    }

    fn gyro_time(&self) -> &crate::time::GyroTime {
        &self.gyro_time
    }

    fn temperature_readings(&self) -> Vec<TemperatureReading> {
        vec![TemperatureReading::new(
            TemperatureSensor::SiaFilter,
            self.sia_fil_temp,
        )]
    }

    fn raw_angle_data(&self) -> Option<AngleData> {
        Some(AngleData::from_raw_counts(
            self.angle_x,
            self.angle_y,
            self.angle_z,
        ))
    }

    fn filtered_angle_data(&self) -> Option<AngleData> {
        None // Raw messages don't have filtered data
    }
}

// SAFETY: RawGyroInertialData is repr(C, packed) and all fields are Pod
unsafe impl Pod for RawGyroInertialData {}
// SAFETY: RawGyroInertialData is repr(C, packed) and all fields are Zeroable
unsafe impl Zeroable for RawGyroInertialData {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_size() {
        assert_eq!(std::mem::size_of::<RawGyroInertialData>(), 26);
        assert_eq!(RawGyroInertialData::PACKET_SIZE, 26);
    }
}
