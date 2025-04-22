//! Noise simulation module for sensor modeling
//!
//! This module provides functions for generating realistic sensor noise
//! for astronomical images.

use std::time::Duration;

use ndarray::Array2;
use rand::rngs::StdRng;
use rand::{thread_rng, RngCore, SeedableRng};

use rand_distr::{Distribution, Normal, Poisson};

use crate::SensorConfig;

// These additional imports will be needed if you want to create and seed your own RNG
// use rand::SeedableRng;    // For StdRng::seed_from_u64
// use rand::thread_rng;     // For getting a thread-local RNG

/// Generates a plausible noise field for a sensor with given parameters.
///
/// # Arguments
/// * `sensor` - Configuration of the sensor
/// * `exposure_time` - Exposure time as Duration
/// * `rng_seed` - Optional seed for random number generator
///
/// # Returns
/// * An ndarray::Array2<f64> containing the noise values for each pixel
pub fn generate_sensor_noise(
    sensor: &SensorConfig,
    exposure_time: &Duration,
    rng_seed: Option<u64>,
) -> Array2<f64> {
    // Create a random number generator from the supplied seed
    let rng_seed = rng_seed.unwrap_or(thread_rng().next_u64());
    let mut rng = StdRng::seed_from_u64(rng_seed);

    // Normal distribution for read noise
    let read_noise_dist = Normal::new(sensor.read_noise_e, sensor.read_noise_e.sqrt()).unwrap();

    // Calculate expected dark current electrons during exposure
    let dark_electrons_mean = sensor.dark_current_e_p_s * exposure_time.as_secs_f64();

    // Generate noise for each pixel based on the above distributions
    let mut noise_field =
        Array2::<f64>::zeros((sensor.height_px as usize, sensor.width_px as usize));
    noise_field.mapv_inplace(|_| {
        // For dark_electrons_mean < 0.1, use Gaussian approximation for better numerical stability
        let dark_noise = if dark_electrons_mean < 0.1 {
            // Use Gaussian approximation for very low dark current
            let dark_normal = Normal::new(0.0, dark_electrons_mean.sqrt()).unwrap();
            dark_normal.sample(&mut rng).max(0.0)
        } else {
            // Use Poisson distribution for larger dark current
            let dark_poisson = Poisson::new(dark_electrons_mean).unwrap();
            dark_poisson.sample(&mut rng)
        };

        // Generate read noise (follows normal distribution)
        let read_noise_value = read_noise_dist.sample(&mut rng).max(0.0);

        // Total noise is the sum of dark current and read noise
        dark_noise + read_noise_value
    });

    noise_field
}

pub fn est_noise_floor(
    sensor: &SensorConfig,
    exposure_time: &Duration,
    rng_seed: Option<u64>,
) -> f64 {
    // Create a smaller sensor with the same noise characteristics
    let tiny_sensor = SensorConfig::new(
        sensor.name.clone(),
        sensor.quantum_efficiency.clone(),
        128, // width
        128, // height
        sensor.pixel_size_um,
        sensor.read_noise_e,
        sensor.dark_current_e_p_s,
        sensor.bit_depth,
        sensor.dn_per_electron,
        sensor.max_well_depth_e,
    );

    // Generate noise field using the smaller sensor
    let noise_field = generate_sensor_noise(&tiny_sensor, exposure_time, rng_seed);

    // Return the mean of the noise field as the estimated noise floor
    noise_field
        .mean()
        .expect("Failed to calculate mean noise value?")
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;

    use crate::{photometry::Band, QuantumEfficiency};

    use super::*;

    fn make_tiny_test_sensor(
        size: (usize, usize),
        dark_current: f64,
        read_noise: f64,
    ) -> SensorConfig {
        let band = Band::new(300.0, 700.0);
        let qe = QuantumEfficiency::from_notch(&band, 1.0).unwrap();

        SensorConfig::new(
            "Test Sensor",
            qe,
            size.1 as u32,
            size.0 as u32,
            5.0,
            read_noise,
            dark_current,
            8,
            1.0,
            1.0,
        )
    }

    #[test]
    fn test_est_noise_floor() {
        // Test that the estimated noise floor is close to the expected value
        let shape = (100, 100);
        let read_noise = 5.0;
        let dark_current = 10.0;
        let exposure_time = Duration::from_secs(1);

        let sensor = make_tiny_test_sensor(shape, dark_current, read_noise);

        let noise_floor = est_noise_floor(&sensor, &exposure_time, Some(42));

        // Calculate the expected noise floor (mean)
        let expected_noise_floor = read_noise + dark_current * exposure_time.as_secs_f64();

        // Assert that the estimated noise floor is close to the expected value
        assert_relative_eq!(noise_floor, expected_noise_floor, epsilon = 0.05);
    }

    #[test]
    fn test_generate_sensor_noise_dimensions() {
        // Test that the generated noise has the correct dimensions
        let shape = (10, 20);
        let read_noise = 2.0;
        let dark_current = 0.1;

        let sensor = make_tiny_test_sensor(shape, dark_current, read_noise);
        let exposure_time = Duration::from_secs(1);

        let noise = generate_sensor_noise(&sensor, &exposure_time, None);

        // Check dimensions
        assert_eq!(noise.dim(), shape);

        // Check that all values are positive or zero (noise is clamped to ≥0)
        for value in noise.iter() {
            assert!(*value >= 0.0);
        }
    }

    #[test]
    fn test_generate_sensor_noise_deterministic() {
        // Test that with the same RNG seed, we get the same noise pattern
        let shape = (5, 5);
        let read_noise = 2.0;
        let dark_current = 0.1;
        let exposure_time = Duration::from_secs_f64(1.0);

        let sensor = make_tiny_test_sensor(shape, dark_current, read_noise);

        // Generate noise with both RNGs
        let noise1 = generate_sensor_noise(&sensor, &exposure_time, Some(5));
        let noise2 = generate_sensor_noise(&sensor, &exposure_time, Some(5));

        // Check that the noise patterns are identical
        for (v1, v2) in noise1.iter().zip(noise2.iter()) {
            assert_eq!(*v1, *v2);
        }

        // Check that a different seed produces different noise
        let noise3 = generate_sensor_noise(&sensor, &exposure_time, Some(6));

        // At least one value should be different
        let mut any_different = false;
        for (v1, v3) in noise1.iter().zip(noise3.iter()) {
            if (v1 - v3).abs() > 1e-10 {
                any_different = true;
                break;
            }
        }
        assert!(
            any_different,
            "Noise patterns with different seeds should differ"
        );
    }

    #[test]
    fn test_generate_sensor_noise_zero_exposure() {
        // Test that with zero exposure time, the noise is only read noise
        let shape = (100, 100); // Use a larger shape for better statistics
        let read_noise = 5.0;
        let dark_current = 10.0;
        let exposure_time = Duration::from_secs_f64(0.0);

        // Create a sensor with the specified parameters
        let sensor = make_tiny_test_sensor(shape, dark_current, read_noise);

        let noise = generate_sensor_noise(&sensor, &exposure_time, Some(3));

        // Calculate mean and standard deviation of the noise
        let mean = noise.mean().unwrap();

        // With zero exposure, the mean should be approximately the read noise
        assert_relative_eq!(mean, read_noise, epsilon = 0.1);
    }

    #[test]
    fn test_generate_sensor_noise_grows_linear() {
        // Test that with zero exposure time, the noise is only read noise
        let shape = (100, 100); // Use a larger shape for better statistics
        let read_noise = 5.0;
        let dark_current = 10.0;

        // Create a sensor with the specified parameters
        let sensor = make_tiny_test_sensor(shape, dark_current, read_noise);

        let mean_0 = generate_sensor_noise(&sensor, &Duration::from_secs(0), Some(7))
            .mean()
            .unwrap();
        let mean_1 = generate_sensor_noise(&sensor, &Duration::from_secs(10), Some(8))
            .mean()
            .unwrap();
        let mean_2 = generate_sensor_noise(&sensor, &Duration::from_secs(20), Some(9))
            .mean()
            .unwrap();

        // With zero exposure, the mean should be approximately the read noise
        assert_relative_eq!(mean_1 - mean_0, 100.0, epsilon = 0.1);
        assert_relative_eq!(mean_2 - mean_0, 200.0, epsilon = 0.1);
    }

    #[test]
    fn test_generate_sensor_noise_always_positive() {
        // Test that noise values are always positive, even with zero exposure time
        let shape = (50, 50);
        let read_noise = 5.0;
        let dark_current = 10.0;

        let sensor = make_tiny_test_sensor(shape, dark_current, read_noise);

        // Test with zero exposure time
        let zero_exposure = Duration::from_secs(0);
        let noise_zero = generate_sensor_noise(&sensor, &zero_exposure, Some(42));

        // Check all values are positive
        for value in noise_zero.iter() {
            assert!(
                *value >= 0.0,
                "Noise should never be negative with zero exposure"
            );
        }

        // Test with non-zero exposure time
        let long_exposure = Duration::from_secs(5);
        let noise_long = generate_sensor_noise(&sensor, &long_exposure, Some(43));

        // Check all values are positive
        for value in noise_long.iter() {
            assert!(
                *value >= 0.0,
                "Noise should never be negative with non-zero exposure"
            );
        }
    }
}
