//! Sinusoid fitting for FSM calibration
//!
//! Extracts amplitude and phase from noisy centroid data by fitting a sinusoidal model.

use ndarray::Array1;
use std::f64::consts::PI;

/// Result of fitting a sinusoid to data
#[derive(Debug, Clone)]
pub struct SinusoidFit {
    /// Amplitude of the fitted sinusoid
    pub amplitude: f64,
    /// Phase offset in radians
    pub phase: f64,
    /// DC offset (mean value)
    pub offset: f64,
    /// Coefficient of determination (R²), 0.0 to 1.0
    pub r_squared: f64,
}

/// Errors that can occur during sinusoid fitting
#[derive(Debug, Clone, PartialEq)]
pub enum FitError {
    /// Not enough data points to fit
    InsufficientData { expected: usize, got: usize },
    /// Data and time arrays have different lengths
    LengthMismatch { data_len: usize, time_len: usize },
    /// Fit quality too low (non-sinusoidal data)
    LowFitQuality { r_squared: f64, threshold: f64 },
    /// Zero variance in data (constant signal)
    ZeroVariance,
}

impl std::fmt::Display for FitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FitError::InsufficientData { expected, got } => {
                write!(
                    f,
                    "insufficient data: expected at least {expected}, got {got}"
                )
            }
            FitError::LengthMismatch { data_len, time_len } => {
                write!(
                    f,
                    "length mismatch: data has {data_len} points, time has {time_len}"
                )
            }
            FitError::LowFitQuality {
                r_squared,
                threshold,
            } => {
                write!(
                    f,
                    "fit quality too low: R²={r_squared:.4}, threshold={threshold:.4}"
                )
            }
            FitError::ZeroVariance => write!(f, "data has zero variance"),
        }
    }
}

impl std::error::Error for FitError {}

/// Minimum number of data points required for fitting
const MIN_DATA_POINTS: usize = 10;

/// Fit a sinusoid to data at a known frequency
///
/// Uses least squares fitting to find amplitude and phase of a sinusoid
/// at the specified frequency. The model is:
///
/// `y(t) = offset + amplitude * sin(2π * frequency * t + phase)`
///
/// # Arguments
/// * `data` - Measured values (e.g., centroid positions)
/// * `time_s` - Timestamps in seconds
/// * `frequency` - Known frequency of the excitation signal in Hz
///
/// # Returns
/// * `Ok(SinusoidFit)` - Fitted parameters
/// * `Err(FitError)` - If fitting fails
pub fn fit_sinusoid(data: &[f64], time_s: &[f64], frequency: f64) -> Result<SinusoidFit, FitError> {
    // Validate inputs
    if data.len() != time_s.len() {
        return Err(FitError::LengthMismatch {
            data_len: data.len(),
            time_len: time_s.len(),
        });
    }

    if data.len() < MIN_DATA_POINTS {
        return Err(FitError::InsufficientData {
            expected: MIN_DATA_POINTS,
            got: data.len(),
        });
    }

    let n = data.len() as f64;
    let omega = 2.0 * PI * frequency;

    // Convert to ndarray for vectorized operations
    let data = Array1::from_vec(data.to_vec());
    let time = Array1::from_vec(time_s.to_vec());

    // Compute DC offset (mean) and center the data
    let offset = data.mean().unwrap_or(0.0);
    let data_centered = &data - offset;

    // Compute sine and cosine basis functions
    let sin_basis = (&time * omega).mapv(f64::sin);
    let cos_basis = (&time * omega).mapv(f64::cos);

    // Least squares: project data onto sin and cos basis
    // a = (2/n) * sum(data * sin)
    // b = (2/n) * sum(data * cos)
    let a = (&data_centered * &sin_basis).sum() * 2.0 / n;
    let b = (&data_centered * &cos_basis).sum() * 2.0 / n;

    // Amplitude and phase from a*sin + b*cos = A*sin(wt + phi)
    let amplitude = (a * a + b * b).sqrt();
    let phase = b.atan2(a);

    // Compute R² (coefficient of determination)
    // Total sum of squares
    let ss_tot = (&data_centered * &data_centered).sum();

    if ss_tot < f64::EPSILON {
        return Err(FitError::ZeroVariance);
    }

    // Fitted values and residuals
    let fitted = (&time * omega + phase).mapv(f64::sin) * amplitude;
    let residuals = &data_centered - &fitted;
    let ss_res = (&residuals * &residuals).sum();

    let r_squared = 1.0 - ss_res / ss_tot;

    Ok(SinusoidFit {
        amplitude,
        phase,
        offset,
        r_squared,
    })
}

/// Fit sinusoid with minimum R² threshold
///
/// Same as [`fit_sinusoid`] but returns an error if R² is below threshold.
///
/// # Arguments
/// * `data` - Measured values
/// * `time_s` - Timestamps in seconds
/// * `frequency` - Known frequency in Hz
/// * `min_r_squared` - Minimum acceptable R² value
///
/// # Returns
/// * `Ok(SinusoidFit)` - If fit succeeds and R² >= threshold
/// * `Err(FitError)` - If fitting fails or R² is too low
pub fn fit_sinusoid_checked(
    data: &[f64],
    time_s: &[f64],
    frequency: f64,
    min_r_squared: f64,
) -> Result<SinusoidFit, FitError> {
    let fit = fit_sinusoid(data, time_s, frequency)?;

    if fit.r_squared < min_r_squared {
        return Err(FitError::LowFitQuality {
            r_squared: fit.r_squared,
            threshold: min_r_squared,
        });
    }

    Ok(fit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn generate_sinusoid(
        amplitude: f64,
        frequency: f64,
        phase: f64,
        offset: f64,
        times: &[f64],
    ) -> Vec<f64> {
        let omega = 2.0 * PI * frequency;
        times
            .iter()
            .map(|&t| offset + amplitude * (omega * t + phase).sin())
            .collect()
    }

    fn generate_times(sample_rate: f64, duration: f64) -> Vec<f64> {
        let n = (sample_rate * duration).ceil() as usize;
        (0..n).map(|i| i as f64 / sample_rate).collect()
    }

    #[test]
    fn test_fit_perfect_sinusoid() {
        let amplitude = 100.0;
        let frequency = 1.0;
        let phase = 0.0;
        let offset = 50.0;

        let times = generate_times(100.0, 5.0); // 5 complete cycles
        let data = generate_sinusoid(amplitude, frequency, phase, offset, &times);

        let fit = fit_sinusoid(&data, &times, frequency).unwrap();

        assert_relative_eq!(fit.amplitude, amplitude, epsilon = 1.0);
        assert_relative_eq!(fit.offset, offset, epsilon = 0.1);
        assert!(
            fit.r_squared > 0.99,
            "R² should be very high for perfect data"
        );
    }

    #[test]
    fn test_fit_with_phase_offset() {
        let amplitude = 50.0;
        let frequency = 2.0;
        let phase = PI / 4.0; // 45 degrees
        let offset = 0.0;

        let times = generate_times(200.0, 3.0);
        let data = generate_sinusoid(amplitude, frequency, phase, offset, &times);

        let fit = fit_sinusoid(&data, &times, frequency).unwrap();

        assert_relative_eq!(fit.amplitude, amplitude, epsilon = 1.0);
        assert_relative_eq!(fit.phase, phase, epsilon = 0.1);
        assert!(fit.r_squared > 0.99);
    }

    #[test]
    fn test_fit_with_dc_offset() {
        let amplitude = 30.0;
        let frequency = 1.0;
        let phase = 0.0;
        let offset = 200.0;

        let times = generate_times(100.0, 5.0);
        let data = generate_sinusoid(amplitude, frequency, phase, offset, &times);

        let fit = fit_sinusoid(&data, &times, frequency).unwrap();

        assert_relative_eq!(fit.offset, offset, epsilon = 0.5);
        assert_relative_eq!(fit.amplitude, amplitude, epsilon = 1.0);
    }

    #[test]
    fn test_fit_noisy_sinusoid() {
        let amplitude = 100.0;
        let frequency = 1.0;
        let times = generate_times(100.0, 5.0);

        // Add deterministic "noise" using a different frequency
        let noise_amplitude = 5.0;
        let data: Vec<f64> = times
            .iter()
            .map(|&t| {
                amplitude * (2.0 * PI * frequency * t).sin()
                    + noise_amplitude * (2.0 * PI * 7.3 * t).sin()
            })
            .collect();

        let fit = fit_sinusoid(&data, &times, frequency).unwrap();

        // Should still recover amplitude reasonably well
        assert_relative_eq!(fit.amplitude, amplitude, epsilon = 10.0);
        // R² should be lower but still good
        assert!(
            fit.r_squared > 0.9,
            "R² should still be reasonable with noise"
        );
    }

    #[test]
    fn test_fit_non_sinusoidal_data() {
        let times = generate_times(100.0, 5.0);

        // Linear data - not sinusoidal at all
        let data: Vec<f64> = times.iter().map(|&t| t * 10.0).collect();

        let fit = fit_sinusoid(&data, &times, 1.0).unwrap();

        // R² should be low
        assert!(
            fit.r_squared < 0.5,
            "R² should be low for non-sinusoidal data"
        );
    }

    #[test]
    fn test_fit_checked_threshold() {
        let times = generate_times(100.0, 5.0);
        let data: Vec<f64> = times.iter().map(|&t| t * 10.0).collect(); // Linear

        let result = fit_sinusoid_checked(&data, &times, 1.0, 0.95);

        assert!(matches!(result, Err(FitError::LowFitQuality { .. })));
    }

    #[test]
    fn test_insufficient_data() {
        let times = vec![0.0, 0.1, 0.2];
        let data = vec![1.0, 2.0, 3.0];

        let result = fit_sinusoid(&data, &times, 1.0);

        assert!(matches!(
            result,
            Err(FitError::InsufficientData {
                expected: 10,
                got: 3
            })
        ));
    }

    #[test]
    fn test_length_mismatch() {
        let times = vec![0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
        let data = vec![1.0, 2.0, 3.0];

        let result = fit_sinusoid(&data, &times, 1.0);

        assert!(matches!(result, Err(FitError::LengthMismatch { .. })));
    }

    #[test]
    fn test_zero_variance() {
        let times: Vec<f64> = (0..100).map(|i| i as f64 / 100.0).collect();
        let data = vec![5.0; 100]; // Constant data

        let result = fit_sinusoid(&data, &times, 1.0);

        assert!(matches!(result, Err(FitError::ZeroVariance)));
    }

    #[test]
    fn test_negative_phase() {
        let amplitude = 50.0;
        let frequency = 1.0;
        let phase = -PI / 3.0; // -60 degrees
        let offset = 0.0;

        let times = generate_times(100.0, 5.0);
        let data = generate_sinusoid(amplitude, frequency, phase, offset, &times);

        let fit = fit_sinusoid(&data, &times, frequency).unwrap();

        // Phase should be recovered (might wrap around)
        let phase_diff = (fit.phase - phase).abs();
        let phase_diff_wrapped = phase_diff.min(2.0 * PI - phase_diff);
        assert!(
            phase_diff_wrapped < 0.1,
            "Phase should match within wrapping"
        );
    }
}
