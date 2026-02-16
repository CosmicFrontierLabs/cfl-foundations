//! Iterative Closest Point implementation for point cloud alignment
//!
//! This algorithm iteratively matches points between two sets and solves
//! for the optimal rigid transformation (rotation and translation) that
//! aligns them.

mod correspondence;
mod transform;

use nalgebra::{Matrix2, Vector2};
use ndarray::Array2;
use thiserror::Error;

use crate::quaternion::Quaternion;
use correspondence::find_closest_points;
use transform::{
    calculate_error, compute_optimal_transform, convert_to_vector2_points, transform_points,
};

/// Errors that can occur during ICP operations
#[derive(Error, Debug)]
pub enum ICPError {
    #[error("Invalid argument: {0}")]
    ArgumentError(String),

    #[error("SVD decomposition failed to produce U or V^T matrices")]
    SvdFailed,
}

/// Result of ICP algorithm containing transformation parameters and matching points
#[derive(Debug, Clone)]
pub struct ICPResult {
    /// Quaternion representing the rotation component of the transform
    pub rotation_quat: Quaternion,

    /// 2x2 Rotation matrix component of the transform (for compatibility)
    pub rotation: Matrix2<f64>,

    /// Translation vector component of the transform (2x1)
    pub translation: Vector2<f64>,

    /// Matches between source and target point sets as (source_idx, target_idx)
    pub matches: Vec<(usize, usize)>,

    /// Mean squared error of the final alignment
    pub mean_squared_error: f64,

    /// Number of iterations performed
    pub iterations: usize,
}

/// Iterative Closest Point algorithm for aligning two point sets
///
/// # Arguments
/// * `source_points` - Source points as `ndarray::Array2<f64>` with shape [n_points, 2]
/// * `target_points` - Target points as `ndarray::Array2<f64>` with shape [m_points, 2]
/// * `max_iterations` - Maximum number of iterations to perform
/// * `convergence_threshold` - Error threshold for convergence
///
/// # Returns
/// * `Result<ICPResult, ICPError>` - Struct containing transformation parameters and matching information
///
/// # Errors
/// * `ICPError::ArgumentError` - If input arrays don't have 2 columns
/// * `ICPError::SvdFailed` - If SVD decomposition fails during iteration
///
pub fn iterative_closest_point(
    source_points: &Array2<f64>,
    target_points: &Array2<f64>,
    max_iterations: usize,
    convergence_threshold: f64,
) -> Result<ICPResult, ICPError> {
    if source_points.shape()[1] != 2 {
        return Err(ICPError::ArgumentError(
            "Source points must have shape [n_points, 2]".to_string(),
        ));
    }
    if target_points.shape()[1] != 2 {
        return Err(ICPError::ArgumentError(
            "Target points must have shape [m_points, 2]".to_string(),
        ));
    }

    // Convert to Vector2 points for easier manipulation
    let source_vec = convert_to_vector2_points(source_points);
    let target_vec = convert_to_vector2_points(target_points);

    // Initialize transformation
    let mut rotation_quat = Quaternion::identity();
    let mut rotation = Matrix2::identity();
    let mut translation = Vector2::zeros();

    // Current transformed source points (initially just the source points)
    let mut current_source = source_vec.clone();

    // Previous error for convergence check
    let mut prev_error = f64::INFINITY;
    let mut current_error;
    let mut iterations = 0;
    let mut matches = Vec::new();

    for i in 0..max_iterations {
        iterations = i + 1;

        // Find closest points
        matches = find_closest_points(&current_source, &target_vec);

        // Compute optimal transformation
        let (q, t) = compute_optimal_transform(&source_vec, &target_vec, &matches)?;

        // Update transformation
        rotation_quat = q;
        // Extract 2x2 rotation matrix from quaternion for 2D operations
        let full_rotation = q.to_rotation_matrix();
        rotation = Matrix2::new(
            full_rotation[(0, 0)],
            full_rotation[(0, 1)],
            full_rotation[(1, 0)],
            full_rotation[(1, 1)],
        );
        translation = t;

        // Apply transformation to original source points
        current_source = transform_points(&source_vec, &rotation, &translation);

        // Calculate error
        current_error =
            calculate_error(&source_vec, &target_vec, &matches, &rotation, &translation);

        // Check for convergence
        if (prev_error - current_error).abs() < convergence_threshold {
            break;
        }

        prev_error = current_error;
    }

    // Calculate final error
    let final_error = calculate_error(&source_vec, &target_vec, &matches, &rotation, &translation);

    Ok(ICPResult {
        rotation_quat,
        rotation,
        translation,
        matches,
        mean_squared_error: final_error,
        iterations,
    })
}

/// Trait for objects that can be located in a 2D Cartesian coordinate system.
pub trait Locatable2d {
    /// Returns the x-coordinate of the object.
    fn x(&self) -> f64;

    /// Returns the y-coordinate of the object.
    fn y(&self) -> f64;
}

/// Implement Locatable for `nalgebra::Vector2<f64>`
impl Locatable2d for Vector2<f64> {
    fn x(&self) -> f64 {
        self.x
    }

    fn y(&self) -> f64 {
        self.y
    }
}

/// Performs ICP matching between two sets of Locatable2d objects and returns the matched pairs and ICP result.
///
/// # Type Parameters
/// * `R1`: The type of the source objects, must implement `Locatable2d` and `Clone`.
/// * `R2`: The type of the target objects, must implement `Locatable2d` and `Clone`.
///
/// # Arguments
/// * `source` - A slice of source objects. Must not be empty.
/// * `target` - A slice of target objects. Must not be empty.
/// * `max_iterations` - Maximum number of iterations for the ICP algorithm.
/// * `convergence_threshold` - Convergence threshold for the ICP algorithm. Must be positive.
///
/// # Returns
/// * `Result<Vec<(R1, R2)>, ICPError>` - A vector of tuples containing the cloned matched pairs,
///   or an error if the operation fails.
///
/// # Errors
/// * `ICPError::ArgumentError` - If either source or target slice is empty, or if convergence_threshold is not positive.
/// * `ICPError::SvdFailed` - If the SVD decomposition fails during ICP iteration.
pub fn icp_match_objects<R1, R2>(
    source: &[R1],
    target: &[R2],
    max_iterations: usize,
    convergence_threshold: f64,
) -> Result<(Vec<(R1, R2)>, ICPResult), ICPError>
where
    R1: Locatable2d + Clone,
    R2: Locatable2d + Clone,
{
    if source.is_empty() {
        return Err(ICPError::ArgumentError("source slice is empty".to_string()));
    }

    if target.is_empty() {
        return Err(ICPError::ArgumentError("target slice is empty".to_string()));
    }

    if convergence_threshold <= 0.0 {
        return Err(ICPError::ArgumentError(format!(
            "convergence_threshold must be positive, got {convergence_threshold}"
        )));
    }

    let source_points_vec: Vec<f64> = source.iter().flat_map(|p| [p.x(), p.y()]).collect();
    let source_points = Array2::from_shape_vec((source.len(), 2), source_points_vec)
        .expect("Source points vector should have correct length for Array2 conversion");

    let target_points_vec: Vec<f64> = target.iter().flat_map(|p| [p.x(), p.y()]).collect();
    let target_points = Array2::from_shape_vec((target.len(), 2), target_points_vec)
        .expect("Target points vector should have correct length for Array2 conversion");

    let result = iterative_closest_point(
        &source_points,
        &target_points,
        max_iterations,
        convergence_threshold,
    )?;

    let matched_objects: Vec<(R1, R2)> = result
        .matches
        .iter()
        .map(|&(src_idx, tgt_idx)| (source[src_idx].clone(), target[tgt_idx].clone()))
        .collect();

    Ok((matched_objects, result))
}

/// Matches objects from source to target using ICP, returning indices instead of cloned objects.
///
/// This is useful when working with types that don't implement Clone, such as `Box<dyn Trait>`.
///
/// # Arguments
/// * `source` - Slice of source objects implementing Locatable2d
/// * `target` - Slice of target objects implementing Locatable2d
/// * `max_iterations` - Maximum number of ICP iterations
/// * `convergence_threshold` - Minimum mean squared error change to continue iterating
///
/// # Returns
/// * Tuple of (matched_indices, ICPResult) where matched_indices is Vec<(source_idx, target_idx)>
///
/// # Errors
/// * `ICPError::ArgumentError` - If either source or target slice is empty, or if convergence_threshold is not positive.
/// * `ICPError::SvdFailed` - If the SVD decomposition fails during ICP iteration.
pub fn icp_match_indices<R1, R2>(
    source: &[R1],
    target: &[R2],
    max_iterations: usize,
    convergence_threshold: f64,
) -> Result<(Vec<(usize, usize)>, ICPResult), ICPError>
where
    R1: Locatable2d,
    R2: Locatable2d,
{
    if source.is_empty() {
        return Err(ICPError::ArgumentError("source slice is empty".to_string()));
    }

    if target.is_empty() {
        return Err(ICPError::ArgumentError("target slice is empty".to_string()));
    }

    if convergence_threshold <= 0.0 {
        return Err(ICPError::ArgumentError(format!(
            "convergence_threshold must be positive, got {convergence_threshold}"
        )));
    }

    let source_points_vec: Vec<f64> = source.iter().flat_map(|p| [p.x(), p.y()]).collect();
    let source_points = Array2::from_shape_vec((source.len(), 2), source_points_vec)
        .expect("Source points vector should have correct length for Array2 conversion");

    let target_points_vec: Vec<f64> = target.iter().flat_map(|p| [p.x(), p.y()]).collect();
    let target_points = Array2::from_shape_vec((target.len(), 2), target_points_vec)
        .expect("Target points vector should have correct length for Array2 conversion");

    let result = iterative_closest_point(
        &source_points,
        &target_points,
        max_iterations,
        convergence_threshold,
    )?;

    Ok((result.matches.clone(), result))
}

#[cfg(test)]
mod tests;
