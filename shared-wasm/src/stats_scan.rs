//! StatsScan - A struct for computing statistics over floating point data
//!
//! This struct computes min, max, mean, and optionally variance/std_dev from a slice of
//! floating point numbers. Min, max, and mean are computed in a single pass, while
//! variance requires a second pass with the data.
//!
//! Handles NaN detection and returns errors when NaN values are encountered.

use num_traits::float::Float;
use std::fmt;
use thiserror::Error;

/// Error types for StatsScan operations
#[derive(Error, Debug, Clone, PartialEq)]
pub enum StatsError {
    #[error("NaN value encountered at index {0}")]
    NaNEncountered(usize),
    #[error("No data provided (empty slice)")]
    NoData,
}

/// A scanner for statistics over floating point data
///
/// Computes min, max, sum, and count in a single pass. Mean is derived from sum/count.
/// Variance and standard deviation require a second pass with the original data.
#[derive(Debug, Clone)]
pub struct StatsScan<T: Float> {
    min_value: Option<T>,
    max_value: Option<T>,
    sum: T,
    count: usize,
    nan_index: Option<usize>,
}

impl<T: Float + fmt::Debug> StatsScan<T> {
    /// Create a new StatsScan by computing statistics from a slice of values
    ///
    /// This performs a single pass to compute min, max, sum, and count.
    /// If any NaN values are encountered, scanning stops and has_nan() returns true.
    ///
    /// # Arguments
    /// * `data` - A slice of floating point values
    pub fn new(data: &[T]) -> Self {
        let mut min_value = None;
        let mut max_value = None;
        let mut sum = T::zero();
        let mut count = 0usize;
        let mut nan_index = None;

        for (index, &value) in data.iter().enumerate() {
            if value.is_nan() && nan_index.is_none() {
                nan_index = Some(index);
                break;
            }

            sum = sum + value;
            count += 1;

            match (min_value, max_value) {
                (None, None) => {
                    min_value = Some(value);
                    max_value = Some(value);
                }
                (Some(min), Some(max)) => {
                    if value < min {
                        min_value = Some(value);
                    }
                    if value > max {
                        max_value = Some(value);
                    }
                }
                _ => unreachable!("min and max should always be in sync"),
            }
        }

        Self {
            min_value,
            max_value,
            sum,
            count,
            nan_index,
        }
    }

    /// Get the minimum value
    ///
    /// # Returns
    /// * `Ok(T)` - The minimum value if data was provided and no NaN was found
    /// * `Err(StatsError::NaNEncountered(index))` - If NaN values were encountered
    /// * `Err(StatsError::NoData)` - If no data was provided
    pub fn min(&self) -> Result<T, StatsError> {
        if let Some(index) = self.nan_index {
            Err(StatsError::NaNEncountered(index))
        } else {
            self.min_value.ok_or(StatsError::NoData)
        }
    }

    /// Get the maximum value
    ///
    /// # Returns
    /// * `Ok(T)` - The maximum value if data was provided and no NaN was found
    /// * `Err(StatsError::NaNEncountered(index))` - If NaN values were encountered
    /// * `Err(StatsError::NoData)` - If no data was provided
    pub fn max(&self) -> Result<T, StatsError> {
        if let Some(index) = self.nan_index {
            Err(StatsError::NaNEncountered(index))
        } else {
            self.max_value.ok_or(StatsError::NoData)
        }
    }

    /// Get both min and max values as a tuple
    ///
    /// # Returns
    /// * `Ok((T, T))` - A tuple of (min, max) if data was provided and no NaN was found
    /// * `Err(StatsError::NaNEncountered(index))` - If NaN values were encountered
    /// * `Err(StatsError::NoData)` - If no data was provided
    pub fn min_max(&self) -> Result<(T, T), StatsError> {
        Ok((self.min()?, self.max()?))
    }

    /// Get the arithmetic mean of the data
    ///
    /// # Returns
    /// * `Ok(T)` - The mean value if data was provided and no NaN was found
    /// * `Err(StatsError::NaNEncountered(index))` - If NaN values were encountered
    /// * `Err(StatsError::NoData)` - If no data was provided
    pub fn mean(&self) -> Result<T, StatsError> {
        if let Some(index) = self.nan_index {
            Err(StatsError::NaNEncountered(index))
        } else if self.count == 0 {
            Err(StatsError::NoData)
        } else {
            Ok(self.sum / T::from(self.count).unwrap())
        }
    }

    /// Get the sum of all values
    ///
    /// # Returns
    /// * `Ok(T)` - The sum if no NaN was found
    /// * `Err(StatsError::NaNEncountered(index))` - If NaN values were encountered
    /// * `Err(StatsError::NoData)` - If no data was provided
    pub fn sum(&self) -> Result<T, StatsError> {
        if let Some(index) = self.nan_index {
            Err(StatsError::NaNEncountered(index))
        } else if self.count == 0 {
            Err(StatsError::NoData)
        } else {
            Ok(self.sum)
        }
    }

    /// Get the count of values processed
    pub fn count(&self) -> usize {
        self.count
    }

    /// Check if NaN values were encountered during computation
    pub fn has_nan(&self) -> bool {
        self.nan_index.is_some()
    }

    /// Compute the population variance (second pass required)
    ///
    /// This performs a second pass over the data to compute variance.
    /// The mean from the first pass is used for efficiency.
    ///
    /// # Arguments
    /// * `data` - The same slice of floating point values used in `new()`
    ///
    /// # Returns
    /// * `Ok(T)` - The population variance
    /// * `Err(StatsError::NaNEncountered(index))` - If NaN was found in first pass
    /// * `Err(StatsError::NoData)` - If no data was provided
    pub fn variance(&self, data: &[T]) -> Result<T, StatsError> {
        let mean = self.mean()?;
        let n = T::from(self.count).unwrap();

        let sum_squared_diff = data
            .iter()
            .take(self.count) // Only iterate up to count (in case NaN stopped early)
            .map(|&x| {
                let diff = x - mean;
                diff * diff
            })
            .fold(T::zero(), |acc, x| acc + x);

        Ok(sum_squared_diff / n)
    }

    /// Compute the population standard deviation (second pass required)
    ///
    /// This is the square root of the variance.
    ///
    /// # Arguments
    /// * `data` - The same slice of floating point values used in `new()`
    ///
    /// # Returns
    /// * `Ok(T)` - The population standard deviation
    /// * `Err(StatsError::NaNEncountered(index))` - If NaN was found in first pass
    /// * `Err(StatsError::NoData)` - If no data was provided
    pub fn std_dev(&self, data: &[T]) -> Result<T, StatsError> {
        Ok(self.variance(data)?.sqrt())
    }

    /// Get min, max, and mean as a tuple (all from first pass)
    ///
    /// # Returns
    /// * `Ok((T, T, T))` - A tuple of (min, max, mean)
    /// * `Err(StatsError::NaNEncountered(index))` - If NaN values were encountered
    /// * `Err(StatsError::NoData)` - If no data was provided
    pub fn min_max_mean(&self) -> Result<(T, T, T), StatsError> {
        Ok((self.min()?, self.max()?, self.mean()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_stats_f64() {
        let scanner = StatsScan::<f64>::new(&[3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0]);

        assert_eq!(scanner.min().unwrap(), 1.0);
        assert_eq!(scanner.max().unwrap(), 9.0);
        assert_eq!(scanner.min_max().unwrap(), (1.0, 9.0));
        assert_eq!(scanner.count(), 8);

        // Mean = (3+1+4+1+5+9+2+6)/8 = 31/8 = 3.875
        assert!((scanner.mean().unwrap() - 3.875).abs() < 1e-10);
        assert!((scanner.sum().unwrap() - 31.0).abs() < 1e-10);
    }

    #[test]
    fn test_basic_stats_f32() {
        let scanner = StatsScan::<f32>::new(&[3.0, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0]);

        assert_eq!(scanner.min().unwrap(), 1.0);
        assert_eq!(scanner.max().unwrap(), 9.0);
        assert!((scanner.mean().unwrap() - 3.875).abs() < 1e-5);
    }

    #[test]
    fn test_variance_and_std_dev() {
        // Using values with known variance: [2, 4, 4, 4, 5, 5, 7, 9]
        // Mean = 5, Variance = 4, StdDev = 2
        let data = [2.0_f64, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let scanner = StatsScan::new(&data);

        assert!((scanner.mean().unwrap() - 5.0).abs() < 1e-10);
        assert!((scanner.variance(&data).unwrap() - 4.0).abs() < 1e-10);
        assert!((scanner.std_dev(&data).unwrap() - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_nan_handling() {
        let scanner = StatsScan::<f64>::new(&[1.0, 2.0, f64::NAN, 3.0, 4.0]);

        assert!(scanner.has_nan());
        assert_eq!(scanner.min(), Err(StatsError::NaNEncountered(2)));
        assert_eq!(scanner.max(), Err(StatsError::NaNEncountered(2)));
        assert_eq!(scanner.mean(), Err(StatsError::NaNEncountered(2)));
    }

    #[test]
    fn test_all_nan() {
        let scanner = StatsScan::<f64>::new(&[f64::NAN, f64::NAN, f64::NAN]);

        assert!(scanner.has_nan());
        assert_eq!(scanner.min(), Err(StatsError::NaNEncountered(0)));
        assert_eq!(scanner.max(), Err(StatsError::NaNEncountered(0)));
    }

    #[test]
    fn test_no_data() {
        let scanner = StatsScan::<f64>::new(&[]);

        assert_eq!(scanner.min(), Err(StatsError::NoData));
        assert_eq!(scanner.max(), Err(StatsError::NoData));
        assert_eq!(scanner.mean(), Err(StatsError::NoData));
        assert_eq!(scanner.count(), 0);
    }

    #[test]
    fn test_single_value() {
        let data = [42.0_f64];
        let scanner = StatsScan::new(&data);

        assert_eq!(scanner.min().unwrap(), 42.0);
        assert_eq!(scanner.max().unwrap(), 42.0);
        assert_eq!(scanner.mean().unwrap(), 42.0);
        assert_eq!(scanner.count(), 1);
        assert!((scanner.variance(&data).unwrap()).abs() < 1e-10); // Variance of single value is 0
    }

    #[test]
    fn test_negative_values() {
        let scanner = StatsScan::<f64>::new(&[-5.0, -1.0, -10.0, -3.0]);

        assert_eq!(scanner.min().unwrap(), -10.0);
        assert_eq!(scanner.max().unwrap(), -1.0);
        // Mean = (-5-1-10-3)/4 = -19/4 = -4.75
        assert!((scanner.mean().unwrap() - (-4.75)).abs() < 1e-10);
    }

    #[test]
    fn test_infinity_values() {
        let scanner = StatsScan::<f64>::new(&[1.0, f64::INFINITY, -f64::INFINITY, 5.0]);

        assert_eq!(scanner.min().unwrap(), -f64::INFINITY);
        assert_eq!(scanner.max().unwrap(), f64::INFINITY);
        // Mean with infinities is NaN due to infinity - infinity
        assert!(scanner.mean().unwrap().is_nan());
    }

    #[test]
    fn test_min_max_mean_tuple() {
        let scanner = StatsScan::<f64>::new(&[1.0, 2.0, 3.0, 4.0, 5.0]);

        let (min, max, mean) = scanner.min_max_mean().unwrap();
        assert_eq!(min, 1.0);
        assert_eq!(max, 5.0);
        assert!((mean - 3.0).abs() < 1e-10);
    }
}
