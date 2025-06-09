//! Dark current estimation for different sensor temperatures
//!
//! This module provides functionality for estimating dark current at various
//! temperatures based on sensor specifications and thermal models.

/// Dark current estimator that uses a reference temperature and dark current
/// to predict values at other temperatures using the rule of thumb that
/// dark current doubles for every 8°C temperature increase.
#[derive(Debug, Clone, PartialEq)]
pub struct DarkCurrentEstimator {
    /// Reference dark current in electrons/pixel/second
    reference_dark_current: f64,
    /// Reference temperature in degrees Celsius
    reference_temp_c: f64,
}

impl DarkCurrentEstimator {
    /// Creates a new dark current estimator with reference values
    ///
    /// # Arguments
    /// * `reference_dark_current` - Dark current in electrons/pixel/second at reference temperature
    /// * `reference_temp_c` - Reference temperature in degrees Celsius
    ///
    /// # Example
    /// ```
    /// use simulator::hardware::dark_current::DarkCurrentEstimator;
    ///
    /// let estimator = DarkCurrentEstimator::new(0.1, 20.0);
    /// ```
    pub fn new(reference_dark_current: f64, reference_temp_c: f64) -> Self {
        Self {
            reference_dark_current,
            reference_temp_c,
        }
    }

    /// Estimates dark current at a target temperature
    ///
    /// # Arguments
    /// * `target_temp_c` - Target temperature in degrees Celsius
    ///
    /// # Returns
    /// Estimated dark current in electrons/pixel/second at target temperature
    ///
    /// # Example
    /// ```
    /// use simulator::hardware::dark_current::DarkCurrentEstimator;
    ///
    /// let estimator = DarkCurrentEstimator::new(0.1, 20.0);
    /// let dark_current_at_28c = estimator.estimate_at_temperature(28.0);
    /// // Should be ~0.2 e-/pixel/s (doubled for 8°C increase)
    /// ```
    pub fn estimate_at_temperature(&self, target_temp_c: f64) -> f64 {
        let temp_diff = target_temp_c - self.reference_temp_c;
        let doubling_periods = temp_diff / 8.0;

        self.reference_dark_current * 2.0_f64.powf(doubling_periods)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_same_temperature() {
        let estimator = DarkCurrentEstimator::new(0.1, 20.0);
        let result = estimator.estimate_at_temperature(20.0);
        assert_eq!(result, 0.1);
    }

    #[test]
    fn test_8_degree_increase_doubles() {
        let estimator = DarkCurrentEstimator::new(0.1, 20.0);
        let result = estimator.estimate_at_temperature(28.0);
        assert_relative_eq!(result, 0.2, epsilon = 1e-10);
    }

    #[test]
    fn test_8_degree_decrease_halves() {
        let estimator = DarkCurrentEstimator::new(0.1, 20.0);
        let result = estimator.estimate_at_temperature(12.0);
        assert_relative_eq!(result, 0.05, epsilon = 1e-10);
    }

    #[test]
    fn test_16_degree_increase_quadruples() {
        let estimator = DarkCurrentEstimator::new(0.1, 20.0);
        let result = estimator.estimate_at_temperature(36.0);
        assert_relative_eq!(result, 0.4, epsilon = 1e-10);
    }

    #[test]
    fn test_16_degree_decrease_quarters() {
        let estimator = DarkCurrentEstimator::new(0.1, 20.0);
        let result = estimator.estimate_at_temperature(4.0);
        assert_relative_eq!(result, 0.025, epsilon = 1e-10);
    }

    #[test]
    fn test_4_degree_increase_sqrt2() {
        let estimator = DarkCurrentEstimator::new(0.1, 20.0);
        let result = estimator.estimate_at_temperature(24.0);
        // 4°C is half of 8°C, so should be 2^0.5 = sqrt(2) times the reference
        let expected = 0.1 * 2.0_f64.sqrt();
        assert_relative_eq!(result, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_negative_reference_temperature() {
        let estimator = DarkCurrentEstimator::new(0.04, -40.0);
        let result_at_ref = estimator.estimate_at_temperature(-40.0);
        assert_eq!(result_at_ref, 0.04);

        // Test 8°C warmer from -40°C to -32°C should double
        let result_warmer = estimator.estimate_at_temperature(-32.0);
        assert_relative_eq!(result_warmer, 0.08, epsilon = 1e-10);
    }

    #[test]
    fn test_large_temperature_difference() {
        let estimator = DarkCurrentEstimator::new(0.001, -20.0);
        // From -20°C to 60°C is 80°C difference = 10 doubling periods
        // Should be 0.001 * 2^10 = 0.001 * 1024 = 1.024
        let result = estimator.estimate_at_temperature(60.0);
        assert_relative_eq!(result, 1.024, epsilon = 1e-10);
    }

    #[test]
    fn test_fractional_doubling_periods() {
        let estimator = DarkCurrentEstimator::new(1.0, 0.0);
        // 6°C increase = 6/8 = 0.75 doubling periods
        // Should be 1.0 * 2^0.75 = 1.0 * 1.6817928... ≈ 1.6818
        let result = estimator.estimate_at_temperature(6.0);
        let expected = 2.0_f64.powf(0.75);
        assert_relative_eq!(result, expected, epsilon = 1e-10);
    }

    #[test]
    fn test_different_reference_values() {
        // Test with different reference dark currents
        let high_dc = DarkCurrentEstimator::new(10.0, 25.0);
        let low_dc = DarkCurrentEstimator::new(0.001, 25.0);

        // Both should scale by same factor for same temperature change
        let temp_change = 33.0; // 8°C increase, should double
        let high_result = high_dc.estimate_at_temperature(temp_change);
        let low_result = low_dc.estimate_at_temperature(temp_change);

        assert_relative_eq!(high_result, 20.0, epsilon = 1e-10);
        assert_relative_eq!(low_result, 0.002, epsilon = 1e-10);
    }
}
