//! Noise generation utilities for astronomical image processing.
//!
//! Provides essential noise modeling primitives for detector simulation including:
//! - Gaussian noise generation with controlled statistical properties
//! - Poisson photon noise for realistic light arrival statistics
//! - Precomputed parameter noise generation for batch processing
//!
//! # Core Functions
//!
//! ## Simple Normal Array
//! Generate deterministic Gaussian noise fields for testing and validation.
//! Useful for unit tests requiring reproducible noise patterns.
//!
//! ## Noise Generation with Precomputed Parameters
//! Optimized noise generation when sensor parameters are known in advance.
//! Automatically selects between Gaussian (low dark current) and Poisson
//! (high dark current) models for statistical accuracy.
//!
//! ## Poisson Photon Noise
//! Apply realistic photon arrival statistics to mean electron images.
//! Essential for accurate modeling of shot noise in astronomical observations.
//!
//! # Performance
//!
//! All functions utilize parallel processing via rayon for efficient
//! generation of large noise fields. Per-chunk RNG is `SmallRng`
//! (xoshiro256++) — see `algo::parallel` for the rationale. On a 5-core
//! aarch64 box this lands a 61 MP Normal field in ~25 ms (~2.4 GP/s).
//!
//! # Usage
//!
//! Generate realistic sensor noise for astronomical detector simulation.
//! Use generate_sensor_noise for full sensor modeling or generate_noise_with_precomputed_params
//! for batch processing with known parameters.

use crate::algo::process_array_in_parallel_chunks;
use ndarray::Array2;
use rand::{rng, RngCore, SeedableRng};
use rand_distr::{Distribution, Normal, Poisson};

/// Generate a 2D array of normally distributed values for testing purposes.
///
/// This function creates a deterministic array filled with values sampled from
/// a normal (Gaussian) distribution. It's specifically designed for unit tests
/// and simulation validation where reproducible noise patterns are needed.
///
/// # Design Purpose
/// - Provides controlled, repeatable noise for algorithm testing
/// - Avoids statistical variance in test assertions
/// - Enables debugging with consistent noise patterns
///
/// # Arguments
/// * `size` - Tuple of (height, width) for the output array dimensions
/// * `mean` - Mean value of the normal distribution
/// * `std_dev` - Standard deviation of the normal distribution
/// * `seed` - Random seed for deterministic output
///
/// # Returns
/// A 2D array with values sampled from Normal(mean, std_dev)
///
///
/// # Use Cases
/// - Unit testing algorithms that need controlled noise input
/// - Generating reproducible test data for CI/CD pipelines
/// - Creating reference noise patterns for validation
pub fn simple_normal_array(
    size: (usize, usize),
    mean: f64,
    std_dev: f64,
    seed: u64,
) -> Array2<f64> {
    use rand::rngs::StdRng;

    let mut rng = StdRng::seed_from_u64(seed);
    let normal_dist = Normal::new(mean, std_dev)
        .expect("Normal distribution parameters must be valid (std_dev > 0)");
    Array2::from_shape_fn(size, |_| normal_dist.sample(&mut rng))
}

/// Generate noise using Gaussian approximation for both read noise and dark current
fn generate_gaussian_noise(
    width: usize,
    height: usize,
    read_noise: f64,
    dark_current_mean: f64,
    rng_seed: u64,
) -> Array2<f64> {
    // Create the array with zeros
    let noise_field = Array2::<f64>::zeros((height, width));

    // Process the array in parallel chunks with our helper function
    process_array_in_parallel_chunks(
        noise_field,
        rng_seed,
        Some(64), // Process 64 rows at a time
        |chunk, rng| {
            // Create distributions
            let read_noise_dist = Normal::new(read_noise, read_noise.sqrt())
                .expect("Read noise parameters must be valid (read_noise >= 0)");
            let dark_noise_dist = Normal::new(0.0, dark_current_mean.sqrt())
                .expect("Dark current parameters must be valid (dark_current_mean >= 0)");

            // Fill the chunk with noise values
            chunk.iter_mut().for_each(|pixel| {
                let dark_noise = dark_noise_dist.sample(rng).max(0.0);
                let read_noise_value = read_noise_dist.sample(rng).max(0.0);
                *pixel = dark_noise + read_noise_value;
            });
        },
    )
}

/// Generate noise using Poisson distribution for dark current and Gaussian for read noise
fn generate_poisson_noise(
    width: usize,
    height: usize,
    read_noise: f64,
    dark_current_mean: f64,
    rng_seed: u64,
) -> Array2<f64> {
    // Create the array with zeros
    let noise_field = Array2::<f64>::zeros((height, width));

    // Process the array in parallel chunks with our helper function
    process_array_in_parallel_chunks(
        noise_field,
        rng_seed,
        Some(64), // Process 64 rows at a time
        |chunk, rng| {
            // Create distributions
            let read_noise_dist = Normal::new(read_noise, read_noise.sqrt())
                .expect("Read noise parameters must be valid (read_noise >= 0)");
            let dark_poisson = Poisson::new(dark_current_mean)
                .expect("Poisson parameter must be valid (dark_current_mean >= 0)");

            // Fill the chunk with noise values
            chunk.iter_mut().for_each(|pixel| {
                let dark_noise = dark_poisson.sample(rng);
                let read_noise_value = read_noise_dist.sample(rng).max(0.0);
                *pixel = dark_noise + read_noise_value;
            });
        },
    )
}

/// Generate sensor noise with precomputed parameters for batch processing.
///
/// Optimized function for generating multiple noise realizations with the same
/// sensor characteristics. Avoids repeated sensor parameter lookups and is
/// ideal for Monte Carlo simulations or batch image processing.
///
/// # Arguments
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels  
/// * `read_noise` - Read noise RMS in electrons
/// * `dark_current_mean` - Expected dark electrons per pixel
/// * `rng_seed` - Optional seed for reproducibility
///
/// # Returns
/// 2D noise field in electrons with specified characteristics
///
/// # Performance
/// ~2-3x faster than full sensor model when called repeatedly
/// with same parameters.
///
/// # Usage
/// Generate sensor noise with precomputed parameters for batch processing.
/// Ideal for Monte Carlo simulations or repeated noise generation.
pub fn generate_noise_with_precomputed_params(
    width: usize,
    height: usize,
    read_noise: f64,
    dark_current_mean: f64,
    rng_seed: Option<u64>,
) -> Array2<f64> {
    let seed = rng_seed.unwrap_or(rng().next_u64());

    if dark_current_mean < 0.1 {
        generate_gaussian_noise(width, height, read_noise, dark_current_mean, seed)
    } else {
        generate_poisson_noise(width, height, read_noise, dark_current_mean, seed)
    }
}

/// Add zero-mean Gaussian read noise to an electron image in parallel.
///
/// Read noise is the electronics-domain noise of a CCD/CMOS sensor — Johnson
/// noise in the source-follower, kTC reset noise, ADC quantisation. Unlike
/// Poisson shot noise it is *additive*, signal-independent, and well modelled
/// by a zero-mean Gaussian with a per-pixel RMS in electrons. After sampling
/// we clamp negatives to zero, since downstream consumers (quantisers, FITS
/// writers, downstream Poisson-resampling) treat the array as an
/// electron count.
///
/// Implementation mirrors [`apply_poisson_photon_noise`]: the image is split
/// into `chunk_size` row blocks, each block gets a deterministic per-chunk
/// `SmallRng` (xoshiro256++) derived from the base seed, and the Gaussian
/// draws happen inside rayon's parallel iterator. This pattern keeps the
/// output bit-identical for a given seed regardless of thread count.
///
/// # Arguments
/// * `electron_image` - 2D array containing pre-read-noise electron counts
///   per pixel. Consumed by this function; if the caller still needs the
///   pre-noise version they must clone it explicitly before the call.
/// * `read_noise_rms` - RMS of the per-pixel Gaussian, in electrons. `0.0`
///   short-circuits and returns the input unchanged.
/// * `rng_seed` - Optional seed for the per-chunk RNGs
///
/// # Returns
/// * An `ndarray::Array2<f64>` with read noise added, clamped at zero
pub fn apply_gaussian_read_noise(
    electron_image: Array2<f64>,
    read_noise_rms: f64,
    rng_seed: Option<u64>,
) -> Array2<f64> {
    // Zero RMS is a no-op — return the input unchanged. No allocation.
    if read_noise_rms <= 0.0 {
        return electron_image;
    }
    let seed = rng_seed.unwrap_or(rng().next_u64());
    let normal =
        Normal::new(0.0_f64, read_noise_rms).expect("read noise RMS must be finite and >= 0");

    process_array_in_parallel_chunks(
        electron_image,
        seed,
        Some(64), // Match apply_poisson_photon_noise: 64 rows per chunk.
        |chunk_view, rng| {
            chunk_view.iter_mut().for_each(|pixel| {
                *pixel = (*pixel + normal.sample(rng)).max(0.0);
            });
        },
    )
}

/// Apply Poisson arrival time statistics to star photon image in parallel
///
/// This function takes a mean electron image (star flux) and applies Poisson noise
/// to simulate realistic photon arrival statistics. Each pixel's value is treated
/// as the mean of a Poisson distribution.
///
/// # Arguments
/// * `mean_electron_image` - 2D array containing mean electron counts per
///   pixel. Consumed by this function; if the caller still needs the
///   pre-noise version they must clone it explicitly before the call.
/// * `rng_seed` - Optional seed for random number generator
///
/// # Returns
/// * An `ndarray::Array2<f64>` with Poisson-sampled electron counts
pub fn apply_poisson_photon_noise(
    mean_electron_image: Array2<f64>,
    rng_seed: Option<u64>,
) -> Array2<f64> {
    let seed = rng_seed.unwrap_or(rng().next_u64());

    // Process the array in parallel chunks with our helper function
    process_array_in_parallel_chunks(
        mean_electron_image,
        seed,
        Some(64), // Process 64 rows at a time
        |chunk_view, rng| {
            // Apply Poisson sampling to each pixel
            chunk_view.iter_mut().for_each(|pixel| {
                let mean_electrons = *pixel;
                if mean_electrons > 0.0 {
                    // For very small means, use Gaussian approximation to avoid numerical issues
                    let sampled_electrons = if mean_electrons < 20.0 {
                        // Use Poisson distribution directly
                        let poisson = Poisson::new(mean_electrons)
                            .expect("Poisson parameter must be valid (mean_electrons > 0)");
                        poisson.sample(rng)
                    } else {
                        // For large means, use normal approximation (faster and numerically stable)
                        let normal = Normal::new(mean_electrons, mean_electrons.sqrt())
                            .expect("Normal parameters must be valid (mean_electrons > 0)");
                        normal.sample(rng).max(0.0)
                    };
                    *pixel = sampled_electrons;
                } else {
                    // Zero mean means zero photons
                    *pixel = 0.0;
                }
            });
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_doctest_simple_normal_array() {
        // Test the example from the simple_normal_array function documentation
        // Create 10x10 array with mean=100, std_dev=10, seed=42
        let noise = simple_normal_array((10, 10), 100.0, 10.0, 42);
        assert_eq!(noise.dim(), (10, 10));

        // Test statistical properties with larger array for better accuracy
        let large_noise = simple_normal_array((100, 100), 50.0, 5.0, 123);
        let mean_actual = large_noise.mean().unwrap();
        let std_actual = large_noise.std(0.0);

        // Should be approximately correct for large arrays
        assert_relative_eq!(mean_actual, 50.0, epsilon = 0.5);
        assert_relative_eq!(std_actual, 5.0, epsilon = 0.5);
    }

    #[test]
    fn test_deterministic_output() {
        // Same seed should produce same output
        let noise1 = simple_normal_array((5, 5), 0.0, 1.0, 42);
        let noise2 = simple_normal_array((5, 5), 0.0, 1.0, 42);

        for i in 0..5 {
            for j in 0..5 {
                assert_eq!(noise1[[i, j]], noise2[[i, j]]);
            }
        }
    }

    #[test]
    fn apply_gaussian_read_noise_recovers_target_rms() {
        // A large flat image with a known pedestal: after adding RMS=8 e-
        // Gaussian noise, the empirical std should land near 8.
        let pedestal = 500.0_f64;
        let rms = 8.0_f64;
        let image = Array2::from_elem((200, 200), pedestal);
        let noisy = apply_gaussian_read_noise(image, rms, Some(7));

        let mean_actual = noisy.mean().unwrap();
        let std_actual = noisy.std(0.0);
        assert_relative_eq!(mean_actual, pedestal, epsilon = 0.5);
        assert_relative_eq!(std_actual, rms, epsilon = 0.3);
    }

    #[test]
    fn apply_gaussian_read_noise_is_deterministic_for_seed() {
        // Same seed => bit-identical output (regardless of thread count).
        let image = Array2::from_elem((64, 64), 100.0);
        let a = apply_gaussian_read_noise(image.clone(), 5.0, Some(42));
        let b = apply_gaussian_read_noise(image, 5.0, Some(42));
        assert_eq!(a, b);
    }

    #[test]
    fn apply_gaussian_read_noise_zero_rms_is_identity() {
        // RMS=0 short-circuits — every pixel unchanged.
        let image = Array2::from_elem((16, 16), 42.0);
        let expected = image.clone();
        let out = apply_gaussian_read_noise(image, 0.0, Some(0));
        assert_eq!(expected, out);
    }

    #[test]
    fn apply_gaussian_read_noise_clamps_at_zero() {
        // A zero-pedestal image with a large RMS will see negative draws on
        // ~half the pixels; the clamp must drop them to exactly 0.0.
        let image = Array2::from_elem((128, 128), 0.0);
        let out = apply_gaussian_read_noise(image, 5.0, Some(11));
        let min = out.iter().cloned().fold(f64::INFINITY, f64::min);
        assert!(min >= 0.0, "expected non-negative output, got min = {min}");
        // At zero pedestal and RMS=5, about half of draws would be negative;
        // after clamping the empirical mean is RMS/sqrt(2π) ≈ 2.0 e-, much
        // larger than any plausible numerical drift.
        let mean_actual = out.mean().unwrap();
        assert_relative_eq!(
            mean_actual,
            5.0 / (2.0 * std::f64::consts::PI).sqrt(),
            epsilon = 0.25
        );
    }
}
