//! Rigid transform estimation via SVD decomposition.
//!
//! Given matched point correspondences, computes the optimal rotation
//! and translation that minimizes alignment error.

use nalgebra::{Matrix2, Vector2, Vector3};
use ndarray::Array2;

use super::ICPError;
use crate::quaternion::Quaternion;

/// Calculates the geometric centroid (center of mass) of a point set.
pub(super) fn calculate_centroid(points: &[Vector2<f64>]) -> Result<Vector2<f64>, ICPError> {
    if points.is_empty() {
        return Err(ICPError::ArgumentError(
            "cannot compute centroid of empty point set".to_string(),
        ));
    }

    let mut centroid = Vector2::zeros();
    for point in points {
        centroid += point;
    }

    Ok(centroid / points.len() as f64)
}

/// Computes optimal rotation (as quaternion) and translation using SVD.
pub(super) fn compute_optimal_transform(
    source_points: &[Vector2<f64>],
    target_points: &[Vector2<f64>],
    matches: &[(usize, usize)],
) -> Result<(Quaternion, Vector2<f64>), ICPError> {
    let mut src_matched = Vec::with_capacity(matches.len());
    let mut tgt_matched = Vec::with_capacity(matches.len());

    for &(src_idx, tgt_idx) in matches {
        src_matched.push(source_points[src_idx]);
        tgt_matched.push(target_points[tgt_idx]);
    }

    // Compute centroids
    let source_centroid = calculate_centroid(&src_matched)?;
    let target_centroid = calculate_centroid(&tgt_matched)?;

    // Compute covariance matrix
    let mut h = Matrix2::zeros();

    for i in 0..src_matched.len() {
        let p_src_centered = src_matched[i] - source_centroid;
        let p_tgt_centered = tgt_matched[i] - target_centroid;

        h += p_src_centered * p_tgt_centered.transpose();
    }

    // Perform SVD
    let svd = h.svd(true, true);
    let u = svd.u.ok_or(ICPError::SvdFailed)?;
    let v_t = svd.v_t.ok_or(ICPError::SvdFailed)?;

    // Compute rotation matrix
    let mut r = v_t.transpose() * u.transpose();

    // Handle reflection case
    if r.determinant() < 0.0 {
        let mut v_t_fixed = v_t;
        v_t_fixed[(0, 1)] = -v_t_fixed[(0, 1)];
        v_t_fixed[(1, 1)] = -v_t_fixed[(1, 1)];
        r = v_t_fixed.transpose() * u.transpose();
    }

    // Convert 2D rotation matrix to quaternion (z-axis rotation)
    let angle = (r[(1, 0)]).atan2(r[(0, 0)]);
    let axis = Vector3::new(0.0, 0.0, 1.0);
    let q = Quaternion::from_axis_angle(&axis, angle);

    // Compute translation
    let t = target_centroid - r * source_centroid;

    Ok((q, t))
}

/// Converts ndarray point representation to nalgebra Vector2 format.
///
/// Input must have shape [n_points, 2] where each row is [x, y].
pub(super) fn convert_to_vector2_points(points: &Array2<f64>) -> Vec<Vector2<f64>> {
    let mut result = Vec::with_capacity(points.shape()[0]);

    for i in 0..points.shape()[0] {
        result.push(Vector2::new(points[(i, 0)], points[(i, 1)]));
    }

    result
}

/// Applies rigid transformation (rotation + translation) to a set of points.
///
/// Each point is transformed according to: p' = R × p + t
pub(super) fn transform_points(
    points: &[Vector2<f64>],
    rotation: &Matrix2<f64>,
    translation: &Vector2<f64>,
) -> Vec<Vector2<f64>> {
    let mut transformed = Vec::with_capacity(points.len());

    for p in points {
        transformed.push(rotation * p + translation);
    }

    transformed
}

/// Calculates the mean squared error between transformed source points and their matched targets.
///
/// MSE = (1/n) × Σ||R × p_src + t - p_tgt||²
pub(super) fn calculate_error(
    source_points: &[Vector2<f64>],
    target_points: &[Vector2<f64>],
    matches: &[(usize, usize)],
    rotation: &Matrix2<f64>,
    translation: &Vector2<f64>,
) -> f64 {
    let mut total_error = 0.0;

    for &(src_idx, tgt_idx) in matches {
        let p_src = source_points[src_idx];
        let p_tgt = target_points[tgt_idx];

        let p_transformed = rotation * p_src + translation;
        let error = (p_transformed - p_tgt).norm_squared();

        total_error += error;
    }

    if matches.is_empty() {
        return f64::INFINITY;
    }

    total_error / matches.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use std::f64::consts::PI;

    fn rotation_matrix(angle: f64) -> Matrix2<f64> {
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        Matrix2::new(cos_a, -sin_a, sin_a, cos_a)
    }

    fn z_rotation_quaternion(angle: f64) -> Quaternion {
        let axis = Vector3::new(0.0, 0.0, 1.0);
        Quaternion::from_axis_angle(&axis, angle)
    }

    /// Run ICP transform solver with known correspondences (bypasses matching step).
    fn run_icp_with_known_matches(
        source: &Array2<f64>,
        target: &Array2<f64>,
        custom_matches: &[(usize, usize)],
    ) -> super::super::ICPResult {
        assert_eq!(source.shape()[1], 2);
        assert_eq!(target.shape()[1], 2);

        let source_vec = convert_to_vector2_points(source);
        let target_vec = convert_to_vector2_points(target);

        let (rotation_quat, translation) =
            compute_optimal_transform(&source_vec, &target_vec, custom_matches).unwrap();

        let full_rotation = rotation_quat.to_rotation_matrix();
        let rotation = Matrix2::new(
            full_rotation[(0, 0)],
            full_rotation[(0, 1)],
            full_rotation[(1, 0)],
            full_rotation[(1, 1)],
        );

        let error = calculate_error(
            &source_vec,
            &target_vec,
            custom_matches,
            &rotation,
            &translation,
        );

        super::super::ICPResult {
            rotation_quat,
            rotation,
            translation,
            matches: custom_matches.to_vec(),
            mean_squared_error: error,
            iterations: 1,
        }
    }

    #[test]
    fn test_centroid_basic() {
        let points = vec![
            Vector2::new(0.0, 0.0),
            Vector2::new(2.0, 0.0),
            Vector2::new(0.0, 2.0),
            Vector2::new(2.0, 2.0),
        ];
        let centroid = calculate_centroid(&points).unwrap();
        assert_relative_eq!(centroid, Vector2::new(1.0, 1.0), epsilon = 1e-10);
    }

    #[test]
    fn test_centroid_empty() {
        let points: Vec<Vector2<f64>> = vec![];
        let result = calculate_centroid(&points);
        assert!(result.is_err());
    }

    #[test]
    fn test_transform_points_identity() {
        let points = vec![Vector2::new(1.0, 2.0), Vector2::new(3.0, 4.0)];
        let result = transform_points(&points, &Matrix2::identity(), &Vector2::zeros());
        assert_eq!(result, points);
    }

    #[test]
    fn test_icp_translation_only() {
        let source = Array2::from_shape_vec(
            (5, 2),
            vec![0.0, 0.0, 1.0, 0.0, 0.0, 2.0, -1.5, 0.0, 0.0, -1.0],
        )
        .unwrap();

        let translation = Vector2::new(2.0, 3.0);
        let mut target = Array2::zeros((5, 2));
        let matches: Vec<(usize, usize)> = (0..5).map(|i| (i, i)).collect();

        for i in 0..5 {
            let p = Vector2::new(source[(i, 0)], source[(i, 1)]);
            let p_trans = p + translation;
            target[(i, 0)] = p_trans[0];
            target[(i, 1)] = p_trans[1];
        }

        let result = run_icp_with_known_matches(&source, &target, &matches);

        assert_relative_eq!(result.rotation, Matrix2::identity(), epsilon = 1e-4);
        assert_relative_eq!(result.translation, translation, epsilon = 1e-4);
        assert_relative_eq!(result.mean_squared_error, 0.0, epsilon = 1e-4);

        let identity_quat = Quaternion::identity();
        assert_relative_eq!(result.rotation_quat.w, identity_quat.w, epsilon = 1e-4);
    }

    #[test]
    fn test_icp_rotation_only() {
        let source = Array2::from_shape_vec(
            (5, 2),
            vec![0.0, 0.0, 1.0, 0.0, 0.0, 2.0, -1.5, 0.0, 0.0, -1.0],
        )
        .unwrap();

        let angle = PI / 6.0;
        let rotation = rotation_matrix(angle);
        let expected_quat = z_rotation_quaternion(angle);
        let mut target = Array2::zeros((5, 2));
        let matches: Vec<(usize, usize)> = (0..5).map(|i| (i, i)).collect();

        for i in 0..5 {
            let p = Vector2::new(source[(i, 0)], source[(i, 1)]);
            let p_rot = rotation * p;
            target[(i, 0)] = p_rot[0];
            target[(i, 1)] = p_rot[1];
        }

        let result = run_icp_with_known_matches(&source, &target, &matches);

        assert_relative_eq!(result.rotation, rotation, epsilon = 1e-4);
        assert_relative_eq!(result.translation.norm(), 0.0, epsilon = 1e-4);
        assert_relative_eq!(result.mean_squared_error, 0.0, epsilon = 1e-4);
        assert_relative_eq!(result.rotation_quat.w, expected_quat.w, epsilon = 1e-4);
        assert_relative_eq!(result.rotation_quat.z, expected_quat.z, epsilon = 1e-4);
    }

    #[test]
    fn test_icp_rotation_and_translation() {
        let source = Array2::from_shape_vec(
            (5, 2),
            vec![0.0, 0.0, 1.0, 0.0, 0.0, 2.0, -1.5, 0.0, 0.0, -1.0],
        )
        .unwrap();

        let angle = PI / 4.0;
        let rotation = rotation_matrix(angle);
        let expected_quat = z_rotation_quaternion(angle);
        let translation = Vector2::new(2.0, 1.0);
        let mut target = Array2::zeros((5, 2));
        let matches: Vec<(usize, usize)> = (0..5).map(|i| (i, i)).collect();

        for i in 0..5 {
            let p = Vector2::new(source[(i, 0)], source[(i, 1)]);
            let p_transformed = rotation * p + translation;
            target[(i, 0)] = p_transformed[0];
            target[(i, 1)] = p_transformed[1];
        }

        let result = run_icp_with_known_matches(&source, &target, &matches);

        assert_relative_eq!(result.rotation, rotation, epsilon = 1e-4);
        assert_relative_eq!(result.translation, translation, epsilon = 1e-4);
        assert_relative_eq!(result.mean_squared_error, 0.0, epsilon = 1e-4);
        assert_relative_eq!(result.rotation_quat.w, expected_quat.w, epsilon = 1e-4);
        assert_relative_eq!(result.rotation_quat.z, expected_quat.z, epsilon = 1e-4);
    }
}
