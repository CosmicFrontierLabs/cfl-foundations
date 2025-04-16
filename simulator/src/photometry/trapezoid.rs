//! Trapezoidal integration utility

use thiserror::Error;

/// Errors that can occur during trapezoidal integration
#[derive(Debug, Error)]
pub enum TrapezoidError {
    #[error("Insufficient points for integration, need at least 2 points")]
    InsufficientPoints,

    #[error("Points must be in ascending order")]
    NotAscending,
}

/// Performs trapezoidal integration of a function over a set of points.
///
/// # Arguments
///
/// * `corners` - The x coordinates of the trapezoid corners in ascending order
/// * `to_integrate` - The function to integrate
///
/// # Returns
///
/// The result of the trapezoidal integration or an error if the input is invalid.
pub fn trap_integrate<F>(corners: Vec<f32>, to_integrate: F) -> Result<f32, TrapezoidError>
where
    F: Fn(f32) -> f32,
{
    // Need at least 2 points to define a trapezoid
    if corners.len() < 2 {
        return Err(TrapezoidError::InsufficientPoints);
    }

    // Verify points are in ascending order
    for i in 1..corners.len() {
        if corners[i] <= corners[i - 1] {
            return Err(TrapezoidError::NotAscending);
        }
    }

    let mut sum = 0.0;

    // Calculate each trapezoid area
    for i in 0..corners.len() - 1 {
        let x1 = corners[i];
        let x2 = corners[i + 1];
        let y1 = to_integrate(x1);
        let y2 = to_integrate(x2);

        // Area of trapezoid = (base) * (average height)
        sum += (x2 - x1) * (y1 + y2) / 2.0;
    }

    Ok(sum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trap_integrate() {
        // Integrate f(x) = x^2 from 0 to 3 using 4 points
        // The trapezoidal approximation gives us:
        // (1-0)(0^2+1^2)/2 + (2-1)(1^2+2^2)/2 + (3-2)(2^2+3^2)/2
        // = 0.5 + 2.5 + 6.5 = 9.5
        let corners = vec![0.0, 1.0, 2.0, 3.0];
        let result = trap_integrate(corners, |x| x * x).unwrap();

        assert!((result - 9.5).abs() < 1e-5);
    }

    #[test]
    fn test_insufficient_points() {
        let corners = vec![1.0];
        let result = trap_integrate(corners, |x| x);

        assert!(matches!(result, Err(TrapezoidError::InsufficientPoints)));
    }

    #[test]
    fn test_not_ascending() {
        let corners = vec![0.0, 2.0, 1.0, 3.0];
        let result = trap_integrate(corners, |x| x);

        assert!(matches!(result, Err(TrapezoidError::NotAscending)));
    }
}
