//! Time representation for Exail Asterix NS frames
//!
//! A 4-byte field that can be interpreted as either a single TimeTag (u32)
//! or as a pair of GyroTimeTag and TimeBase (u16, u16).

use bytemuck::{Pod, Zeroable};

/// Time field interpretation mode
///
/// The message_id determines how the 4-byte time field should be interpreted:
/// - BASE variants (17, 19, 21): Use TagAndBase interpretation
/// - Regular variants (18, 20, 22): Use SingleTag interpretation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeInterpretation {
    /// Single u32 TimeTag (regular variants)
    SingleTag(u32),
    /// Pair of u16s: (GyroTimeTag, TimeBase) (BASE variants)
    TagAndBase { gyro_time_tag: u16, time_base: u16 },
}

/// 4-byte time field with dual interpretation
///
/// Can be read as:
/// - Single `u32` TimeTag via [`as_time_tag`](Self::as_time_tag)
/// - Pair of `u16` (GyroTimeTag, TimeBase) via [`as_gyro_time_tag_and_base`](Self::as_gyro_time_tag_and_base)
/// - Interpreted based on message type via [`interpret`](Self::interpret)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
#[repr(C)]
pub struct GyroTime([u8; 4]);

impl GyroTime {
    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }

    /// Interpret as single u32 TimeTag (little-endian)
    pub fn as_time_tag(&self) -> u32 {
        u32::from_le_bytes(self.0)
    }

    /// Interpret as pair of u16s: (GyroTimeTag, TimeBase) (little-endian)
    pub fn as_gyro_time_tag_and_base(&self) -> (u16, u16) {
        let gyro_time_tag = u16::from_le_bytes([self.0[0], self.0[1]]);
        let time_base = u16::from_le_bytes([self.0[2], self.0[3]]);
        (gyro_time_tag, time_base)
    }

    /// Raw bytes access
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }

    /// Interpret the time field based on whether this is a BASE variant message
    ///
    /// - BASE variants (is_base_variant = true): Returns TagAndBase
    /// - Regular variants (is_base_variant = false): Returns SingleTag
    pub fn interpret(&self, is_base_variant: bool) -> TimeInterpretation {
        if is_base_variant {
            let (gyro_time_tag, time_base) = self.as_gyro_time_tag_and_base();
            TimeInterpretation::TagAndBase {
                gyro_time_tag,
                time_base,
            }
        } else {
            TimeInterpretation::SingleTag(self.as_time_tag())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as_time_tag() {
        let time = GyroTime::from_bytes([0x12, 0x34, 0x56, 0x78]);
        assert_eq!(time.as_time_tag(), 0x78563412);
    }

    #[test]
    fn test_as_gyro_time_tag_and_base() {
        let time = GyroTime::from_bytes([0x12, 0x34, 0x56, 0x78]);
        let (gyro_time_tag, time_base) = time.as_gyro_time_tag_and_base();
        assert_eq!(gyro_time_tag, 0x3412);
        assert_eq!(time_base, 0x7856);
    }

    #[test]
    fn test_bytes_roundtrip() {
        let bytes = [0xAA, 0xBB, 0xCC, 0xDD];
        let time = GyroTime::from_bytes(bytes);
        assert_eq!(time.as_bytes(), &bytes);
    }

    #[test]
    fn test_interpret_base_variant() {
        let time = GyroTime::from_bytes([0x12, 0x34, 0x56, 0x78]);
        let interp = time.interpret(true);
        match interp {
            TimeInterpretation::TagAndBase {
                gyro_time_tag,
                time_base,
            } => {
                assert_eq!(gyro_time_tag, 0x3412);
                assert_eq!(time_base, 0x7856);
            }
            _ => panic!("Expected TagAndBase variant"),
        }
    }

    #[test]
    fn test_interpret_regular_variant() {
        let time = GyroTime::from_bytes([0x12, 0x34, 0x56, 0x78]);
        let interp = time.interpret(false);
        match interp {
            TimeInterpretation::SingleTag(tag) => {
                assert_eq!(tag, 0x78563412);
            }
            _ => panic!("Expected SingleTag variant"),
        }
    }
}
