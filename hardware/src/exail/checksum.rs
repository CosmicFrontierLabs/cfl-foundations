//! Checksum computation for Exail Asterix NS frames
//!
//! The checksum is the sum of all 16-bit DataWords (little-endian),
//! truncated on overflow. Each DataWord consists of two bytes with
//! LSB sent first, followed by MSB.
//!
//! This module provides both standalone functions and a trait for
//! working with checksums on packet structs.

use bytemuck::{bytes_of, bytes_of_mut, Pod};

/// Trait for packets with a trailing 16-bit checksum.
///
/// Provides methods to verify, compute, and update checksums on packet structs.
/// The checksum is assumed to be the last 2 bytes of the struct (little-endian).
///
/// # Requirements
/// - The implementing type must be `Pod` (plain old data)
/// - The checksum field must be the last 2 bytes of the struct
///
/// # Example
/// See unit tests in `checksummed_trait_tests` module for usage examples.
pub trait Checksummed: Pod {
    /// Verify the packet's stored checksum matches the computed value.
    ///
    /// Returns `true` if the checksum is valid, `false` otherwise.
    fn verify_checksum(&self) -> bool {
        let bytes = bytes_of(self);
        verify_checksum_bytes(bytes)
    }

    /// Compute what the checksum should be for this packet.
    ///
    /// This does not modify the packet; it just returns the computed value.
    fn compute_checksum(&self) -> u16 {
        let bytes = bytes_of(self);
        compute_checksum(&bytes[..bytes.len() - 2])
    }

    /// Compute and write the correct checksum to the packet.
    ///
    /// After calling this, `verify_checksum()` will return `true`.
    fn update_checksum(&mut self) {
        let bytes = bytes_of_mut(self);
        let len = bytes.len();
        let checksum = compute_checksum(&bytes[..len - 2]);
        bytes[len - 2..].copy_from_slice(&checksum.to_le_bytes());
    }
}

/// Compute checksum over a byte slice containing little-endian 16-bit words.
///
/// The input length must be even (multiple of 2 bytes).
/// Returns the wrapping sum of all 16-bit little-endian words.
pub fn compute_checksum(data: &[u8]) -> u16 {
    debug_assert!(
        data.len() % 2 == 0,
        "Data length must be even for checksum computation"
    );

    data.chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .fold(0u16, |acc, word| acc.wrapping_add(word))
}

/// Verify that a frame's checksum is valid.
///
/// The frame should include all bytes up to and including the checksum.
/// The checksum of the entire frame (including the checksum field) should
/// equal the checksum field itself when computed correctly.
///
/// For a frame of N bytes where the last 2 bytes are the checksum:
/// - Computes sum of bytes 0..N-2
/// - Compares against the checksum stored in bytes N-2..N
pub fn verify_checksum_bytes(frame: &[u8]) -> bool {
    if frame.len() < 4 || frame.len() % 2 != 0 {
        return false;
    }

    let data_end = frame.len() - 2;
    let computed = compute_checksum(&frame[..data_end]);
    let stored = u16::from_le_bytes([frame[data_end], frame[data_end + 1]]);

    computed == stored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_from_datasheet_example() {
        // Example from datasheet section 4.2.3:
        // FRAME: 12 02 F1 4C 0F B3 00 00 06 07 1E 00 FA 06 41 01 06 F9 BE FE 00 00 00 00 35 09
        // Checksum should be 0x0935
        let frame: [u8; 26] = [
            0x12, 0x02, 0xF1, 0x4C, 0x0F, 0xB3, 0x00, 0x00, 0x06, 0x07, 0x1E, 0x00, 0xFA, 0x06,
            0x41, 0x01, 0x06, 0xF9, 0xBE, 0xFE, 0x00, 0x00, 0x00, 0x00, 0x35, 0x09,
        ];

        // Verify the stored checksum
        let stored_checksum = u16::from_le_bytes([frame[24], frame[25]]);
        assert_eq!(stored_checksum, 0x0935);

        // Compute checksum of data portion (excluding checksum bytes)
        let computed = compute_checksum(&frame[..24]);
        assert_eq!(computed, 0x0935);

        // Verify full frame
        assert!(verify_checksum_bytes(&frame));
    }

    #[test]
    fn test_checksum_computation_step_by_step() {
        // Verify individual DataWord parsing from the example
        // First DW: 12 02 -> 0x0212
        assert_eq!(u16::from_le_bytes([0x12, 0x02]), 0x0212);
        // Second DW: F1 4C -> 0x4CF1
        assert_eq!(u16::from_le_bytes([0xF1, 0x4C]), 0x4CF1);
        // Third DW: 0F B3 -> 0xB30F
        assert_eq!(u16::from_le_bytes([0x0F, 0xB3]), 0xB30F);
    }

    #[test]
    fn test_checksum_wrapping() {
        // Test that overflow wraps correctly
        let data: [u8; 4] = [0xFF, 0xFF, 0x02, 0x00]; // 0xFFFF + 0x0002 = 0x0001 (wrapped)
        assert_eq!(compute_checksum(&data), 0x0001);
    }

    #[test]
    fn test_verify_checksum_bytes_invalid() {
        let mut frame: [u8; 26] = [
            0x12, 0x02, 0xF1, 0x4C, 0x0F, 0xB3, 0x00, 0x00, 0x06, 0x07, 0x1E, 0x00, 0xFA, 0x06,
            0x41, 0x01, 0x06, 0xF9, 0xBE, 0xFE, 0x00, 0x00, 0x00, 0x00, 0x35, 0x09,
        ];

        // Corrupt the checksum
        frame[24] = 0x00;
        assert!(!verify_checksum_bytes(&frame));
    }

    #[test]
    fn test_verify_checksum_bytes_short_frame() {
        assert!(!verify_checksum_bytes(&[0x00, 0x00])); // Too short
        assert!(!verify_checksum_bytes(&[0x00, 0x00, 0x00])); // Odd length
    }

    mod checksummed_trait_tests {
        use super::*;
        use crate::exail::health_status::HealthStatus;
        use crate::exail::messages::{FilteredGyroInertialData, FullGyroData, RawGyroInertialData};
        use crate::exail::time::GyroTime;

        fn make_raw_packet() -> RawGyroInertialData {
            RawGyroInertialData {
                start_word: 0x12,
                message_id: 0x12,
                gyro_time: GyroTime::from_bytes([0; 4]),
                angle_x: 1000,
                angle_y: 2000,
                angle_z: 3000,
                sia_fil_temp: 16384,
                health_status: HealthStatus::FOG_VALIDITY,
                checksum: 0,
            }
        }

        fn make_filtered_packet() -> FilteredGyroInertialData {
            FilteredGyroInertialData {
                start_word: 0x12,
                message_id: 0x14,
                gyro_time: GyroTime::from_bytes([0; 4]),
                angle_x: 1000,
                angle_y: 2000,
                angle_z: 3000,
                sia_fil_temp: 16384,
                health_status: HealthStatus::FOG_VALIDITY,
                checksum: 0,
            }
        }

        fn make_full_packet() -> FullGyroData {
            FullGyroData {
                start_word: 0x12,
                message_id: 0x16,
                gyro_time: GyroTime::from_bytes([0; 4]),
                raw_ang_x: 1000,
                raw_ang_y: 2000,
                raw_ang_z: 3000,
                fil_ang_x: 1000,
                fil_ang_y: 2000,
                fil_ang_z: 3000,
                so_in_cur: 0x1000,
                cur_com: 0x0800,
                pow_meas_x: 0x2000,
                pow_meas_y: 0x2000,
                pow_meas_z: 0x2000,
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
                health_status: HealthStatus::FOG_VALIDITY,
                checksum: 0,
            }
        }

        #[test]
        fn test_raw_packet_update_and_verify() {
            let mut packet = make_raw_packet();
            assert!(!packet.verify_checksum());

            packet.update_checksum();
            assert!(packet.verify_checksum());
        }

        #[test]
        fn test_filtered_packet_update_and_verify() {
            let mut packet = make_filtered_packet();
            assert!(!packet.verify_checksum());

            packet.update_checksum();
            assert!(packet.verify_checksum());
        }

        #[test]
        fn test_full_packet_update_and_verify() {
            let mut packet = make_full_packet();
            assert!(!packet.verify_checksum());

            packet.update_checksum();
            assert!(packet.verify_checksum());
        }

        #[test]
        fn test_compute_checksum_returns_correct_value() {
            let mut packet = make_raw_packet();
            let computed = packet.compute_checksum();

            packet.update_checksum();
            // Copy from packed struct
            let stored = packet.checksum;

            assert_eq!(computed, stored);
        }

        #[test]
        fn test_update_checksum_is_idempotent() {
            let mut packet = make_full_packet();

            packet.update_checksum();
            let checksum1 = packet.checksum;

            packet.update_checksum();
            let checksum2 = packet.checksum;

            assert_eq!(checksum1, checksum2);
        }

        #[test]
        fn test_checksum_changes_with_data() {
            let mut packet1 = make_raw_packet();
            let mut packet2 = make_raw_packet();

            packet1.update_checksum();
            let checksum1 = packet1.checksum;

            packet2.angle_x = 9999;
            packet2.update_checksum();
            let checksum2 = packet2.checksum;

            assert_ne!(checksum1, checksum2);
        }

        #[test]
        fn test_verify_detects_corruption() {
            let mut packet = make_full_packet();
            packet.update_checksum();
            assert!(packet.verify_checksum());

            // Corrupt data after checksum was set
            let bytes = bytemuck::bytes_of_mut(&mut packet);
            bytes[10] ^= 0xFF;

            assert!(!packet.verify_checksum());
        }
    }
}
