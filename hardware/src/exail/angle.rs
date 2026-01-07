//! Angle data decoding for Exail Asterix NS gyro measurements
//!
//! Converts raw u32 counts to angular measurements in arcseconds

/// Conversion factor from LSB to arcseconds
pub const ARCSECONDS_PER_LSB: f64 = 0.00153;

/// Three-axis angle measurements in arcseconds
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AngleData {
    /// X-axis angle in arcseconds
    pub x: f64,
    /// Y-axis angle in arcseconds
    pub y: f64,
    /// Z-axis angle in arcseconds
    pub z: f64,
}

impl AngleData {
    /// Create angle data from raw u32 counts
    pub fn from_raw_counts(x: u32, y: u32, z: u32) -> Self {
        Self {
            x: (x as f64) * ARCSECONDS_PER_LSB,
            y: (y as f64) * ARCSECONDS_PER_LSB,
            z: (z as f64) * ARCSECONDS_PER_LSB,
        }
    }

    /// Convert to degrees
    pub fn to_degrees(&self) -> (f64, f64, f64) {
        const ARCSEC_TO_DEG: f64 = 1.0 / 3600.0;
        (
            self.x * ARCSEC_TO_DEG,
            self.y * ARCSEC_TO_DEG,
            self.z * ARCSEC_TO_DEG,
        )
    }

    /// Convert to radians
    pub fn to_radians(&self) -> (f64, f64, f64) {
        const ARCSEC_TO_RAD: f64 = std::f64::consts::PI / 648000.0;
        (
            self.x * ARCSEC_TO_RAD,
            self.y * ARCSEC_TO_RAD,
            self.z * ARCSEC_TO_RAD,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_angle_conversion() {
        let angles = AngleData::from_raw_counts(1000, 2000, 3000);
        assert_relative_eq!(angles.x, 1.53, epsilon = 1e-10);
        assert_relative_eq!(angles.y, 3.06, epsilon = 1e-10);
        assert_relative_eq!(angles.z, 4.59, epsilon = 1e-10);
    }

    #[test]
    fn test_to_degrees() {
        let angles = AngleData::from_raw_counts(3600000, 7200000, 10800000);
        let (x_deg, y_deg, z_deg) = angles.to_degrees();
        assert_relative_eq!(x_deg, 1.53, epsilon = 1e-6);
        assert_relative_eq!(y_deg, 3.06, epsilon = 1e-6);
        assert_relative_eq!(z_deg, 4.59, epsilon = 1e-6);
    }

    #[test]
    fn test_zero_angles() {
        let angles = AngleData::from_raw_counts(0, 0, 0);
        assert_eq!(angles.x, 0.0);
        assert_eq!(angles.y, 0.0);
        assert_eq!(angles.z, 0.0);
    }
}
