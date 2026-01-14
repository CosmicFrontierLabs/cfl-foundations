//! Sinusoidal command generation for FSM calibration
//!
//! Generates smooth sinusoidal command sequences for driving the FSM during calibration.

use std::f64::consts::PI;

/// Generator for sinusoidal FSM commands
#[derive(Debug, Clone)]
pub struct SinusoidGenerator {
    /// Peak amplitude of the sinusoid
    amplitude: f64,
    /// Frequency in Hz
    frequency: f64,
    /// Sample rate in Hz
    sample_rate: f64,
}

impl SinusoidGenerator {
    /// Create a new sinusoid generator
    ///
    /// # Arguments
    /// * `amplitude` - Peak amplitude of the sinusoid (e.g., microradians)
    /// * `frequency` - Oscillation frequency in Hz
    /// * `sample_rate` - Output sample rate in Hz
    pub fn new(amplitude: f64, frequency: f64, sample_rate: f64) -> Self {
        Self {
            amplitude,
            frequency,
            sample_rate,
        }
    }

    /// Generate a complete sinusoidal sequence for a given duration
    ///
    /// # Arguments
    /// * `duration_s` - Duration of the sequence in seconds
    ///
    /// # Returns
    /// Vector of amplitude values sampled at the configured sample rate
    pub fn generate(&self, duration_s: f64) -> Vec<f64> {
        let num_samples = (duration_s * self.sample_rate).ceil() as usize;
        let dt = 1.0 / self.sample_rate;

        (0..num_samples)
            .map(|i| {
                let t = i as f64 * dt;
                self.sample_at(t)
            })
            .collect()
    }

    /// Sample the sinusoid at a specific time
    ///
    /// # Arguments
    /// * `time_s` - Time in seconds from start
    ///
    /// # Returns
    /// Amplitude value at the given time
    pub fn sample_at(&self, time_s: f64) -> f64 {
        self.amplitude * (2.0 * PI * self.frequency * time_s).sin()
    }

    /// Get the configured amplitude
    pub fn amplitude(&self) -> f64 {
        self.amplitude
    }

    /// Get the configured frequency
    pub fn frequency(&self) -> f64 {
        self.frequency
    }

    /// Get the configured sample rate
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    /// Calculate the period of one complete cycle
    pub fn period(&self) -> f64 {
        1.0 / self.frequency
    }

    /// Generate timestamps for a given duration
    ///
    /// # Arguments
    /// * `duration_s` - Duration in seconds
    ///
    /// # Returns
    /// Vector of timestamps in seconds
    pub fn generate_timestamps(&self, duration_s: f64) -> Vec<f64> {
        let num_samples = (duration_s * self.sample_rate).ceil() as usize;
        let dt = 1.0 / self.sample_rate;

        (0..num_samples).map(|i| i as f64 * dt).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_amplitude_at_peaks() {
        let gen = SinusoidGenerator::new(100.0, 1.0, 1000.0);

        // At t=0.25s (quarter period), sin(π/2) = 1, should be at positive peak
        let quarter_period = 0.25;
        let value = gen.sample_at(quarter_period);
        assert_relative_eq!(value, 100.0, epsilon = 1e-10);

        // At t=0.75s (3/4 period), sin(3π/2) = -1, should be at negative peak
        let three_quarter_period = 0.75;
        let value = gen.sample_at(three_quarter_period);
        assert_relative_eq!(value, -100.0, epsilon = 1e-10);
    }

    #[test]
    fn test_zero_crossings() {
        let gen = SinusoidGenerator::new(100.0, 1.0, 1000.0);

        // At t=0, should be zero
        let value = gen.sample_at(0.0);
        assert_relative_eq!(value, 0.0, epsilon = 1e-10);

        // At t=0.5s (half period), should be zero
        let value = gen.sample_at(0.5);
        assert_relative_eq!(value, 0.0, epsilon = 1e-10);

        // At t=1.0s (full period), should be zero
        let value = gen.sample_at(1.0);
        assert_relative_eq!(value, 0.0, epsilon = 1e-10);
    }

    #[test]
    fn test_frequency_scaling() {
        // 2 Hz frequency means peaks at 0.125s and 0.375s
        let gen = SinusoidGenerator::new(50.0, 2.0, 1000.0);

        // Quarter period at 2 Hz is 0.125s
        let value = gen.sample_at(0.125);
        assert_relative_eq!(value, 50.0, epsilon = 1e-10);

        // Three quarter period at 2 Hz is 0.375s
        let value = gen.sample_at(0.375);
        assert_relative_eq!(value, -50.0, epsilon = 1e-10);
    }

    #[test]
    fn test_generate_sequence_length() {
        let gen = SinusoidGenerator::new(100.0, 1.0, 100.0);

        // 5 seconds at 100 Hz = 500 samples
        let sequence = gen.generate(5.0);
        assert_eq!(sequence.len(), 500);
    }

    #[test]
    fn test_generate_continuity() {
        let gen = SinusoidGenerator::new(100.0, 1.0, 1000.0);
        let sequence = gen.generate(2.0);

        // Check that adjacent samples don't have large jumps
        // For a 1 Hz sinusoid at 1000 Hz sample rate, max change per sample is small
        let max_delta = 2.0 * PI * gen.frequency() * gen.amplitude() / gen.sample_rate();

        for i in 1..sequence.len() {
            let delta = (sequence[i] - sequence[i - 1]).abs();
            assert!(
                delta <= max_delta * 1.01,
                "Discontinuity at sample {i}: delta={delta}, max_expected={max_delta}"
            );
        }
    }

    #[test]
    fn test_period_calculation() {
        let gen = SinusoidGenerator::new(100.0, 2.0, 1000.0);
        assert_relative_eq!(gen.period(), 0.5, epsilon = 1e-10);

        let gen2 = SinusoidGenerator::new(100.0, 0.5, 1000.0);
        assert_relative_eq!(gen2.period(), 2.0, epsilon = 1e-10);
    }

    #[test]
    fn test_timestamps_generation() {
        let gen = SinusoidGenerator::new(100.0, 1.0, 10.0);
        let timestamps = gen.generate_timestamps(1.0);

        assert_eq!(timestamps.len(), 10);
        assert_relative_eq!(timestamps[0], 0.0, epsilon = 1e-10);
        assert_relative_eq!(timestamps[1], 0.1, epsilon = 1e-10);
        assert_relative_eq!(timestamps[9], 0.9, epsilon = 1e-10);
    }
}
