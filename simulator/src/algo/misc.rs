//! Miscellaneous mathematical and utility algorithms.
//!
//! This module provides general-purpose mathematical functions and utilities
//! that don't fit into more specific algorithm categories. Currently includes:
//!
//! - **Linear interpolation**: Fast 1D interpolation with error handling
//! - **Numerical utilities**: Common mathematical operations for scientific computing
//!
//! These functions are designed for performance and robustness in scientific
//! applications, with comprehensive error handling and input validation.

use thiserror::Error;

/// Errors that can occur during interpolation operations.
///
/// This enum provides detailed error information for interpolation failures,
/// allowing callers to handle different error conditions appropriately.
#[derive(Error, Debug)]
pub enum InterpError {
    #[error("Value {0} is out of bounds for interpolation range [{1}, {2}]")]
    OutOfBounds(f64, f64, f64),
    #[error("Input vectors must have at least 2 points")]
    InsufficientData,
    #[error("Input vectors must have the same length")]
    MismatchedLengths,
    #[error("X values must be sorted in ascending order")]
    UnsortedData,
}

/// Performs linear interpolation on 1D data using binary search for efficiency.
///
/// This function implements fast linear interpolation by:
/// 1. Validating input data (lengths, sorting, sufficient points)
/// 2. Using binary search to find the correct interval (O(log n))
/// 3. Applying linear interpolation formula: y = y₁ + t(y₂ - y₁)
///
/// where t = (x - x₁)/(x₂ - x₁) is the interpolation parameter.
///
/// # Arguments
///
/// * `x` - The x-coordinate at which to interpolate
/// * `xs` - Array of x-coordinates (must be sorted in ascending order)
/// * `ys` - Array of corresponding y-values (must match length of xs)
///
/// # Returns
///
/// * `Ok(f64)` - The interpolated y-value at position x
/// * `Err(InterpError)` - Detailed error if interpolation fails
///
/// # Performance
///
/// - Time complexity: O(log n) due to binary search
/// - Space complexity: O(1)
/// - Suitable for repeated queries on the same dataset
///
/// # Examples
///
/// ```rust
/// use simulator::algo::misc::interp;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let x_coords = vec![0.0, 1.0, 2.0, 3.0];
/// let y_values = vec![0.0, 1.0, 4.0, 9.0];
///
/// // Interpolate at x = 1.5
/// let result = interp(1.5, &x_coords, &y_values)?;
/// assert_eq!(result, 2.5);  // Linear interpolation between (1,1) and (2,4)
///
/// // Exact match
/// let exact = interp(2.0, &x_coords, &y_values)?;
/// assert_eq!(exact, 4.0);   // Exact value at x = 2
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// * `InterpError::OutOfBounds` - x is outside the range \\[xs\\[0\\], xs\\[n-1\\]\\]
/// * `InterpError::InsufficientData` - Less than 2 data points provided
/// * `InterpError::MismatchedLengths` - xs and ys have different lengths
/// * `InterpError::UnsortedData` - xs array is not sorted in ascending order
pub fn interp(x: f64, xs: &[f64], ys: &[f64]) -> Result<f64, InterpError> {
    if xs.len() != ys.len() {
        return Err(InterpError::MismatchedLengths);
    }

    if xs.len() < 2 {
        return Err(InterpError::InsufficientData);
    }

    // Check if xs is sorted
    for i in 1..xs.len() {
        if xs[i] <= xs[i - 1] {
            return Err(InterpError::UnsortedData);
        }
    }

    let min_x = xs[0];
    let max_x = xs[xs.len() - 1];

    if x < min_x || x > max_x {
        return Err(InterpError::OutOfBounds(x, min_x, max_x));
    }

    // Binary search for the correct interval
    let idx = match xs.binary_search_by(|probe| probe.partial_cmp(&x).unwrap()) {
        Ok(exact_idx) => return Ok(ys[exact_idx]), // Exact match
        Err(insert_idx) => insert_idx,
    };

    // Linear interpolation between points
    let i1 = idx - 1;
    let i2 = idx;

    let x1 = xs[i1];
    let x2 = xs[i2];
    let y1 = ys[i1];
    let y2 = ys[i2];

    let t = (x - x1) / (x2 - x1);
    Ok(y1 + t * (y2 - y1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let xs = vec![1.0, 2.0, 3.0, 4.0];
        let ys = vec![10.0, 20.0, 30.0, 40.0];
        assert_eq!(interp(2.0, &xs, &ys).unwrap(), 20.0);
    }

    #[test]
    fn test_linear_interpolation() {
        let xs = vec![1.0, 2.0, 3.0];
        let ys = vec![10.0, 20.0, 30.0];
        assert_eq!(interp(1.5, &xs, &ys).unwrap(), 15.0);
        assert_eq!(interp(2.5, &xs, &ys).unwrap(), 25.0);
    }

    #[test]
    fn test_out_of_bounds() {
        let xs = vec![1.0, 2.0, 3.0];
        let ys = vec![10.0, 20.0, 30.0];
        assert!(matches!(
            interp(0.5, &xs, &ys),
            Err(InterpError::OutOfBounds(_, _, _))
        ));
        assert!(matches!(
            interp(3.5, &xs, &ys),
            Err(InterpError::OutOfBounds(_, _, _))
        ));
    }

    #[test]
    fn test_mismatched_lengths() {
        let xs = vec![1.0, 2.0, 3.0];
        let ys = vec![10.0, 20.0];
        assert!(matches!(
            interp(1.5, &xs, &ys),
            Err(InterpError::MismatchedLengths)
        ));
    }

    #[test]
    fn test_insufficient_data() {
        let xs = vec![1.0];
        let ys = vec![10.0];
        assert!(matches!(
            interp(1.0, &xs, &ys),
            Err(InterpError::InsufficientData)
        ));
    }

    #[test]
    fn test_unsorted_data() {
        let xs = vec![2.0, 1.0, 3.0];
        let ys = vec![20.0, 10.0, 30.0];
        assert!(matches!(
            interp(1.5, &xs, &ys),
            Err(InterpError::UnsortedData)
        ));
    }
}
