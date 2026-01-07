//! Optical alignment calibration for camera/display/optics system.
//!
//! Stores the affine transformation between display coordinates and sensor coordinates,
//! as determined by closed-loop calibration.

use nalgebra::{DMatrix, DVector};
use serde::{Deserialize, Serialize};

/// Optical alignment calibration data.
///
/// Represents the affine transformation from display pixel coordinates to sensor pixel coordinates:
/// ```text
/// sensor_x = a * display_x + b * display_y + tx
/// sensor_y = c * display_x + d * display_y + ty
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpticalAlignment {
    /// Affine transform coefficient: x contribution to sensor_x
    pub a: f64,
    /// Affine transform coefficient: y contribution to sensor_x
    pub b: f64,
    /// Affine transform coefficient: x contribution to sensor_y
    pub c: f64,
    /// Affine transform coefficient: y contribution to sensor_y
    pub d: f64,
    /// Translation offset for sensor_x
    pub tx: f64,
    /// Translation offset for sensor_y
    pub ty: f64,
    /// Timestamp when calibration was performed (Unix epoch seconds)
    pub timestamp: u64,
    /// Number of calibration points used
    pub num_points: usize,
    /// RMS residual error in pixels (if computed)
    pub rms_error: Option<f64>,
}

impl OpticalAlignment {
    /// Create a new optical alignment from affine transform parameters
    pub fn new(a: f64, b: f64, c: f64, d: f64, tx: f64, ty: f64, num_points: usize) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            a,
            b,
            c,
            d,
            tx,
            ty,
            timestamp,
            num_points,
            rms_error: None,
        }
    }

    /// Apply transform: convert display coordinates to sensor coordinates
    pub fn display_to_sensor(&self, display_x: f64, display_y: f64) -> (f64, f64) {
        let sensor_x = self.a * display_x + self.b * display_y + self.tx;
        let sensor_y = self.c * display_x + self.d * display_y + self.ty;
        (sensor_x, sensor_y)
    }

    /// Get scale factors (magnitude of column vectors)
    pub fn scale(&self) -> (f64, f64) {
        let scale_x = (self.a * self.a + self.c * self.c).sqrt();
        let scale_y = (self.b * self.b + self.d * self.d).sqrt();
        (scale_x, scale_y)
    }

    /// Get rotation angle in radians
    pub fn rotation(&self) -> f64 {
        self.c.atan2(self.a)
    }

    /// Get rotation angle in degrees
    pub fn rotation_degrees(&self) -> f64 {
        self.rotation().to_degrees()
    }

    /// Save to JSON file
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// Load from JSON file
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let json = std::fs::read_to_string(path)?;
        serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

impl Default for OpticalAlignment {
    /// Default identity transform (no scaling, rotation, or translation)
    fn default() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            tx: 0.0,
            ty: 0.0,
            timestamp: 0,
            num_points: 0,
            rms_error: None,
        }
    }
}

/// A point correspondence for calibration: known source position and measured destination position.
#[derive(Debug, Clone, Copy)]
pub struct PointCorrespondence {
    /// Source X coordinate (e.g., display pixel)
    pub src_x: f64,
    /// Source Y coordinate (e.g., display pixel)
    pub src_y: f64,
    /// Destination X coordinate (e.g., sensor pixel)
    pub dst_x: f64,
    /// Destination Y coordinate (e.g., sensor pixel)
    pub dst_y: f64,
}

impl PointCorrespondence {
    /// Create a new point correspondence
    pub fn new(src_x: f64, src_y: f64, dst_x: f64, dst_y: f64) -> Self {
        Self {
            src_x,
            src_y,
            dst_x,
            dst_y,
        }
    }
}

/// Estimate an affine transformation from point correspondences using SVD least squares.
///
/// Returns the estimated `OpticalAlignment` with RMS error computed, or `None` if
/// estimation fails (e.g., fewer than 3 points or SVD fails).
///
/// # Arguments
/// * `points` - Slice of point correspondences (source -> destination)
pub fn estimate_affine_transform(points: &[PointCorrespondence]) -> Option<OpticalAlignment> {
    let n = points.len();
    if n < 3 {
        return None;
    }

    // Build design matrix A: each row is [src_x, src_y, 1]
    let mut a_data = Vec::with_capacity(n * 3);
    let mut bx = Vec::with_capacity(n);
    let mut by = Vec::with_capacity(n);

    for p in points {
        a_data.push(p.src_x);
        a_data.push(p.src_y);
        a_data.push(1.0);
        bx.push(p.dst_x);
        by.push(p.dst_y);
    }

    let a_matrix = DMatrix::from_row_slice(n, 3, &a_data);
    let bx_vec = DVector::from_vec(bx);
    let by_vec = DVector::from_vec(by);

    // Solve using SVD (robust to ill-conditioned systems)
    let svd = a_matrix.svd(true, true);

    let params_x = svd.solve(&bx_vec, 1e-10).ok()?;
    let params_y = svd.solve(&by_vec, 1e-10).ok()?;

    let mut alignment = OpticalAlignment::new(
        params_x[0], // a
        params_x[1], // b
        params_y[0], // c
        params_y[1], // d
        params_x[2], // tx
        params_y[2], // ty
        n,
    );

    // Compute RMS error
    let mut sum_sq_error = 0.0;
    for p in points {
        let (pred_x, pred_y) = alignment.display_to_sensor(p.src_x, p.src_y);
        let err_x = pred_x - p.dst_x;
        let err_y = pred_y - p.dst_y;
        sum_sq_error += err_x * err_x + err_y * err_y;
    }
    alignment.rms_error = Some((sum_sq_error / n as f64).sqrt());

    Some(alignment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_identity_transform() {
        let align = OpticalAlignment::default();
        let (sx, sy) = align.display_to_sensor(100.0, 200.0);
        assert_relative_eq!(sx, 100.0, epsilon = 1e-10);
        assert_relative_eq!(sy, 200.0, epsilon = 1e-10);
    }

    #[test]
    fn test_scale() {
        let align = OpticalAlignment::new(2.0, 0.0, 0.0, 2.0, 0.0, 0.0, 100);
        let (scale_x, scale_y) = align.scale();
        assert_relative_eq!(scale_x, 2.0, epsilon = 1e-10);
        assert_relative_eq!(scale_y, 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_rotation() {
        // 90 degree rotation: a=0, b=-1, c=1, d=0
        let align = OpticalAlignment::new(0.0, -1.0, 1.0, 0.0, 0.0, 0.0, 100);
        let rot_deg = align.rotation_degrees();
        assert_relative_eq!(rot_deg, 90.0, epsilon = 1e-10);
    }

    #[test]
    fn test_estimate_affine_transform_identity_with_offset() {
        // Points that form an identity transform with offset (10, 20)
        let points = vec![
            PointCorrespondence::new(0.0, 0.0, 10.0, 20.0),
            PointCorrespondence::new(100.0, 0.0, 110.0, 20.0),
            PointCorrespondence::new(0.0, 100.0, 10.0, 120.0),
            PointCorrespondence::new(100.0, 100.0, 110.0, 120.0),
        ];

        let align = estimate_affine_transform(&points).unwrap();

        // Should be ~identity with offset
        assert_relative_eq!(align.a, 1.0, epsilon = 1e-10);
        assert_relative_eq!(align.d, 1.0, epsilon = 1e-10);
        assert!(align.b.abs() < 1e-10);
        assert!(align.c.abs() < 1e-10);
        assert_relative_eq!(align.tx, 10.0, epsilon = 1e-10);
        assert_relative_eq!(align.ty, 20.0, epsilon = 1e-10);

        // RMS error should be ~0
        assert!(align.rms_error.unwrap() < 1e-10);
    }

    #[test]
    fn test_estimate_affine_transform_scale() {
        // Points that form a 2x scale transform
        let points = vec![
            PointCorrespondence::new(0.0, 0.0, 0.0, 0.0),
            PointCorrespondence::new(100.0, 0.0, 200.0, 0.0),
            PointCorrespondence::new(0.0, 100.0, 0.0, 200.0),
            PointCorrespondence::new(100.0, 100.0, 200.0, 200.0),
        ];

        let align = estimate_affine_transform(&points).unwrap();

        let (scale_x, scale_y) = align.scale();
        assert_relative_eq!(scale_x, 2.0, epsilon = 1e-10);
        assert_relative_eq!(scale_y, 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_estimate_affine_transform_too_few_points() {
        let points = vec![
            PointCorrespondence::new(0.0, 0.0, 10.0, 20.0),
            PointCorrespondence::new(100.0, 0.0, 110.0, 20.0),
        ];

        assert!(estimate_affine_transform(&points).is_none());
    }
}
