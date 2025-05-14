//! Zodiacal light brightness model
//!
//! This module provides functionality for interpolating zodiacal light data.
//! It implements a bilinear interpolation approach for estimating zodiacal light brightness
//! at arbitrary ecliptic coordinates.
//! It uses the data from here: https://etc.stsci.edu/etcstatic/users_guide/1_ref_9_background.html#zodiacal-light
//! This data in turn is derived from the work of Leinert et al. (1998) and is available at:
//! https://doi.org/10.1051/aas:1998105
use ndarray::Array2;
use thiserror::Error;

/// Errors that can occur when working with zodiacal light data
#[derive(Error, Debug)]
pub enum ZodicalError {
    #[error("Coordinates out of range: ecliptic longitude {0}, ecliptic latitude {1}")]
    OutOfRange(f64, f64),

    #[error("Interpolation error: {0}")]
    InterpolationError(String),
}

/// Represents zodiacal light brightness data as a function of ecliptic coordinates
pub struct ZodicalLight {
    /// The underlying data table containing brightness values in magnitudes per square arcsecond
    data: Array2<f64>,
}

// Hardcoded ecliptic coordinate grids
const LONGITUDES: [f64; 20] = [
    0.0, 5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 40.0, 45.0, 50.0, 60.0, 75.0, 90.0, 105.0, 120.0,
    135.0, 150.0, 165.0, 180.0,
];

const LATITUDES: [f64; 13] = [
    0.0, 5.0, 10.0, 15.0, 20.0, 25.0, 30.0, 35.0, 45.0, 50.0, 60.0, 75.0, 90.0,
];

// Hardcoded data table (embedded at compile time)
// Each row corresponds to an ecliptic longitude
// Each column corresponds to an ecliptic latitude
// format-off
#[rustfmt::skip]
fn zodical_raw_data() -> [[f64; 13]; 20] {
    [
        [ f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, 22.0708, 22.5136, 22.9538, 23.2298],
        [ f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, 22.0816, 22.5136, 22.9538, 23.2298],
        [ f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, 22.1033, 22.5210, 22.9538, 23.2298],
        [ f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, 22.1454, 22.5360, 22.9538, 23.2298],
        [ f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, 22.2004, 22.5743, 22.9649, 23.2298],
        [ f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, 22.0808,  22.2586, 22.6141, 22.9762, 23.2298],
        [ f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, 22.1578,  22.3237, 22.6554, 23.0107, 23.2298],
        [ f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, 21.9203,  22.2350,  22.3924, 22.7071, 23.0224, 23.2298],
        [ f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN, 21.8257,  22.0287,  22.3181,  22.4628, 22.7522, 23.0343, 23.2298],
        [ f64::NAN, f64::NAN, 21.0810,  21.3356,  21.5717,  21.7872,  21.9545,  22.1379,  22.3948,  22.5232, 22.7801, 23.0707, 23.2298],
        [ 20.8432,  21.0663,  21.3194,  21.5397,  21.7408,  21.9486,  22.0833,  22.2472,  22.4715,  22.5837, 22.8080, 23.1071, 23.2298],
        [ 21.1844,  21.3356,  21.5842,  21.7872,  21.9859,  22.1525,  22.2937,  22.4437,  22.6304,  22.7237, 22.9104, 23.1212, 23.2298],
        [ 21.6258,  21.6965,  21.8737,  22.0611,  22.2180,  22.3621,  22.4989,  22.6319,  22.7801,  22.8542, 23.0024, 23.1607, 23.2298],
        [ 21.9155,  21.9768,  22.0660,  22.2350,  22.3948,  22.5284,  22.6470,  22.7699,  22.9104,  22.9807, 23.1212, 23.2016, 23.2298],
        [ 22.1315,  22.1419,  22.2124,  22.3686,  22.5136,  22.6387,  22.7614,  22.8912,  22.9990,  23.0529, 23.1607, 23.2298, 23.2298],
        [ 22.2639,  22.2757,  22.3305,  22.4844,  22.5980,  22.7071,  22.8186,  22.9646,  23.0707,  23.1237, 23.2298, 23.2736, 23.2298],
        [ 22.3181,  22.3243,  22.3948,  22.5284,  22.6304,  22.7339,  22.8483,  22.9224,  23.0707,  23.1237, 23.2298, 23.2885, 23.2298],
        [ 22.3181,  22.3243,  22.4014,  22.5210,  22.6060,  22.6896,  22.7801,  22.8639,  22.9990,  23.0665, 23.2016, 23.3037, 23.2298],
        [ 22.2180,  22.2407,  22.3181,  22.4014,  22.4989,  22.5743,  22.6554,  22.7435,  22.9104,  22.9938, 23.1607, 23.3037, 23.2298],
        [ 22.0418,  22.1315,  22.2236,  22.3243,  22.4216,  22.5210,  22.6304,  22.7348,  22.8998,  22.9823, 23.1473, 23.3037, 23.2298],
    ]
}

impl ZodicalLight {
    /// Create a new ZodicalLight instance with the embedded data
    pub fn new() -> Self {
        // Convert the hardcoded 2D array to an ndarray::Array2
        let mut data = Array2::zeros((LONGITUDES.len(), LATITUDES.len()));
        for (i, row) in zodical_raw_data().iter().enumerate() {
            for (j, &val) in row.iter().enumerate() {
                data[[i, j]] = val;
            }
        }

        Self { data }
    }

    /// Find the indices and interpolation weights for a value within an array
    ///
    /// # Arguments
    ///
    /// * `array` - Array of coordinate values (sorted)
    /// * `value` - Value to locate in the array
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// * The lower index
    /// * The upper index
    /// * The weight for the lower value (interpolation factor)
    fn find_indices_and_weights(array: &[f64], value: f64) -> Option<(usize, usize, f64)> {
        if array.len() < 2 {
            return None;
        }

        // Handle out of bounds cases
        if value < array[0] || value > array[array.len() - 1] {
            return None;
        }

        // Find the lower index using binary search
        let mut lower_idx = 0;
        let mut upper_idx = array.len() - 1;

        while upper_idx - lower_idx > 1 {
            let mid_idx = (lower_idx + upper_idx) / 2;
            if array[mid_idx] <= value {
                lower_idx = mid_idx;
            } else {
                upper_idx = mid_idx;
            }
        }

        // If we hit the exact value, use the same index for both
        if array[lower_idx] == value {
            return Some((lower_idx, lower_idx, 1.0));
        }

        // Compute the weight for interpolation
        let lower_val = array[lower_idx];
        let upper_val = array[upper_idx];
        let weight = (value - lower_val) / (upper_val - lower_val);

        Some((lower_idx, upper_idx, 1.0 - weight))
    }

    /// Get the zodiacal light brightness at given helio-ecliptic longitude or elongation
    /// (sun angle between sun-earth and earth particle) coordinates using bilinear
    /// interpolation
    ///
    /// # Arguments
    ///
    /// * `longitude` - Helio-ecliptic longitude longitude in degrees (0 to 180)
    /// * `latitude` - Ecliptic latitude in degrees (0 to 90)
    ///
    /// # Returns
    ///
    /// A `Result` containing either the brightness in magnitudes per square arcsecond or an error
    pub fn get_brightness(&self, longitude: f64, latitude: f64) -> Result<f64, ZodicalError> {
        // Find indices and weights for longitude
        let (lon_idx1, lon_idx2, lon_weight) =
            Self::find_indices_and_weights(&LONGITUDES, longitude)
                .ok_or(ZodicalError::OutOfRange(longitude, latitude))?;

        // Find indices and weights for latitude
        let (lat_idx1, lat_idx2, lat_weight) = Self::find_indices_and_weights(&LATITUDES, latitude)
            .ok_or(ZodicalError::OutOfRange(longitude, latitude))?;

        // Get the four corner values
        let q11 = self.data[[lon_idx1, lat_idx1]];
        let q12 = self.data[[lon_idx1, lat_idx2]];
        let q21 = self.data[[lon_idx2, lat_idx1]];
        let q22 = self.data[[lon_idx2, lat_idx2]];

        // Handle NaN values by using nearest non-NaN value
        let mut valid_points = Vec::new();
        let mut valid_weights = Vec::new();

        if !q11.is_nan() {
            valid_points.push(q11);
            valid_weights.push(lon_weight * lat_weight);
        }

        if !q12.is_nan() {
            valid_points.push(q12);
            valid_weights.push(lon_weight * (1.0 - lat_weight));
        }

        if !q21.is_nan() {
            valid_points.push(q21);
            valid_weights.push((1.0 - lon_weight) * lat_weight);
        }

        if !q22.is_nan() {
            valid_points.push(q22);
            valid_weights.push((1.0 - lon_weight) * (1.0 - lat_weight));
        }

        // If we have no valid points, return an error
        if valid_points.is_empty() {
            return Err(ZodicalError::InterpolationError(
                "No valid data points for interpolation".to_string(),
            ));
        }

        // Normalize weights
        let weight_sum: f64 = valid_weights.iter().sum();
        let normalized_weights: Vec<f64> = valid_weights.iter().map(|&w| w / weight_sum).collect();

        // Calculate weighted average
        let result = valid_points
            .iter()
            .zip(normalized_weights.iter())
            .map(|(&val, &weight)| val * weight)
            .sum();

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use super::*;

    #[test]
    fn test_find_indices_and_weights() {
        let array = [0.0, 10.0, 20.0, 30.0, 40.0];

        // Test exact match
        let (idx1, idx2, weight) = ZodicalLight::find_indices_and_weights(&array, 10.0).unwrap();
        assert_eq!(idx1, 1);
        assert_eq!(idx2, 1);
        assert_eq!(weight, 1.0);

        // Test interpolation
        let (idx1, idx2, weight) = ZodicalLight::find_indices_and_weights(&array, 15.0).unwrap();
        assert_eq!(idx1, 1);
        assert_eq!(idx2, 2);
        assert!((weight - 0.5).abs() < 1e-10);

        // Test out of bounds (low)
        assert!(ZodicalLight::find_indices_and_weights(&array, -5.0).is_none());

        // Test out of bounds (high)
        assert!(ZodicalLight::find_indices_and_weights(&array, 45.0).is_none());
    }

    #[test]
    fn test_get_brightness() {
        let zodical = ZodicalLight::new();

        // Test a coordinate with known value (exact match to a grid point)
        let brightness = zodical.get_brightness(90.0, 50.0).unwrap();
        assert!((brightness - 22.9807).abs() < 1e-10);

        // Test interpolation between grid points
        let brightness = zodical.get_brightness(87.5, 47.5).unwrap();
        // NOTE(meawoppl) - this is me eyeballing the chart, so high epsilon
        assert_relative_eq!(brightness, 22.9, epsilon = 0.1);

        // Test out of range
        assert!(zodical.get_brightness(200.0, 50.0).is_err());
        assert!(zodical.get_brightness(90.0, 100.0).is_err());
    }

    #[test]
    fn test_brightness_range() {
        let zodical = ZodicalLight::new();

        // Test a grid of points across the valid range
        // This covers the sun exclusion zone and will have a bunch
        // of points on the cusp to stress the interpolation
        // We expect the brightness to be between 20 and 24 mag
        for lon in (0..=180).step_by(10) {
            for lat in (0..=90).step_by(5) {
                let lon_f64 = lon as f64;
                let lat_f64 = lat as f64;

                if let Ok(brightness) = zodical.get_brightness(lon_f64, lat_f64) {
                    if !brightness.is_nan() {
                        assert!(
                            brightness >= 20.0 && brightness <= 24.0,
                            "Brightness at lon={}, lat={} is {}, which is outside the expected range [20, 24]",
                            lon_f64, lat_f64, brightness
                        );
                    }
                }
            }
        }
    }
}
