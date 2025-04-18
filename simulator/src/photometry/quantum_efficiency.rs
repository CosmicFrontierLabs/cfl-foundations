//! Quantum efficiency modeling for photometric sensors

use thiserror::Error;

use super::Band;

/// Errors that can occur with quantum efficiency calculations
#[derive(Debug, Error)]
pub enum QuantumEfficiencyError {
    #[error("Wavelength and efficiency vectors must have the same length")]
    LengthMismatch,

    #[error("Wavelengths must be in ascending order")]
    NotAscending,

    #[error("First and last efficiency values must be 0.0")]
    BoundaryNotZero,

    #[error("Efficiency values must be between 0.0 and 1.0")]
    OutOfRange,
}

/// Models the quantum efficiency of a sensor across a range of wavelengths
///
/// This struct stores wavelength-efficiency pairs and provides methods to
/// evaluate the efficiency at any wavelength within the defined range.
#[derive(Debug, Clone)]
pub struct QuantumEfficiency {
    /// Wavelengths in nanometers (nm)
    wavelengths: Vec<f64>,

    /// Efficiency values (0.0 to 1.0) corresponding to each wavelength
    efficiencies: Vec<f64>,
}

// TODO(meawoppl) - convert the internal storage to f64

impl QuantumEfficiency {
    /// Create a new QuantumEfficiency model from a explicit notch
    ///
    /// # Arguments
    ///
    /// * `band` - Band that the notch applies to
    /// * `efficiency` - Efficiency value (0.0 to 1.0) for the notch
    ///
    /// # Returns
    /// A Result containing the new QuantumEfficiency or an error
    pub fn from_notch(band: &Band, efficiency: f64) -> Result<Self, QuantumEfficiencyError> {
        // Validate efficiency value
        if !(0.0..=1.0).contains(&efficiency) {
            return Err(QuantumEfficiencyError::OutOfRange);
        }

        let low_nm = band.lower_nm;
        let high_nm = band.upper_nm;

        // This should use f64::next_up() and f64::next_down() but those are unstable
        // This should be small compared to anything we care about with cmos, but
        // large enough to not get eaten by a ULP dumbness anywhere
        let smol = 1e-8;

        // Create the wavelength vector
        let wavelengths = vec![low_nm - smol, low_nm, high_nm, high_nm + smol];
        // Create the efficiency vector with 0.0 at both ends and the notch in the middle
        let efficiencies = vec![0.0, efficiency, efficiency, 0.0];

        // Return the new QuantumEfficiency instance
        Self::from_table(wavelengths, efficiencies)
    }

    /// Create a new QuantumEfficiency model from wavelength and efficiency tables
    ///
    /// # Arguments
    ///
    /// * `wavelengths` - Wavelengths in nanometers, must be in ascending order
    /// * `efficiencies` - Efficiency values (0.0 to 1.0) for each wavelength
    ///
    /// # Returns
    ///
    /// A Result containing the new QuantumEfficiency or an error
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The vectors have different lengths
    /// - Wavelengths are not in ascending order
    /// - First or last efficiency value is not 0.0
    /// - Any efficiency value is outside the range [0.0, 1.0]
    pub fn from_table(
        wavelengths: Vec<f64>,
        efficiencies: Vec<f64>,
    ) -> Result<Self, QuantumEfficiencyError> {
        // Check vectors have the same length
        if wavelengths.len() != efficiencies.len() {
            return Err(QuantumEfficiencyError::LengthMismatch);
        }

        // Check we have at least two points
        if wavelengths.len() < 2 {
            return Err(QuantumEfficiencyError::LengthMismatch);
        }

        // Check wavelengths are in ascending order
        for i in 1..wavelengths.len() {
            if wavelengths[i] <= wavelengths[i - 1] {
                return Err(QuantumEfficiencyError::NotAscending);
            }
        }

        // Check first and last efficiency values are 0.0
        if efficiencies[0] != 0.0 || efficiencies[efficiencies.len() - 1] != 0.0 {
            return Err(QuantumEfficiencyError::BoundaryNotZero);
        }

        // Check all efficiency values are between 0.0 and 1.0
        for &efficiency in &efficiencies {
            if !(0.0..=1.0).contains(&efficiency) {
                return Err(QuantumEfficiencyError::OutOfRange);
            }
        }

        Ok(Self {
            wavelengths,
            efficiencies,
        })
    }

    /// Get the quantum efficiency at a specific wavelength
    ///
    /// If the wavelength is outside the defined range, returns 0.0
    ///
    /// # Arguments
    ///
    /// * `wavelength` - The wavelength in nanometers (nm)
    ///
    /// # Returns
    ///
    /// The interpolated efficiency value (0.0 to 1.0)
    pub fn at(&self, wavelength: f64) -> f64 {
        // Return 0.0 if outside the range
        if wavelength < self.wavelengths[0] || wavelength > *self.wavelengths.last().unwrap() {
            return 0.0;
        }

        // Find the segment that contains the wavelength
        for i in 0..self.wavelengths.len() - 1 {
            if wavelength >= self.wavelengths[i] && wavelength <= self.wavelengths[i + 1] {
                // Linear interpolation
                let t = (wavelength - self.wavelengths[i])
                    / (self.wavelengths[i + 1] - self.wavelengths[i]);

                return self.efficiencies[i] * (1.0 - t) + self.efficiencies[i + 1] * t;
            }
        }

        // Should never reach here if input is in range
        unreachable!()
    }

    /// Returns the band (wavelength range) of the quantum efficiency.
    ///
    /// # Returns
    ///
    /// A `Band` struct containing the lower and upper wavelengths in nanometers.
    pub fn band(&self) -> Band {
        Band {
            lower_nm: self.wavelengths[0],
            upper_nm: *self.wavelengths.last().unwrap(),
        }
    }

    /// Integrate the quantum efficiency over the wavelength range
    ///
    /// # Arguments
    ///
    /// * `f` - Function that takes wavelength (nm) and returns a value to multiply with QE
    ///
    /// # Returns
    ///
    /// The integrated value
    pub fn integrate<F>(&self, f: F) -> f64
    where
        F: Fn(f64) -> f64,
    {
        let mut sum = 0.0;

        // Integrate over each segment
        for i in 0..self.wavelengths.len() - 1 {
            let x1 = self.wavelengths[i];
            let x2 = self.wavelengths[i + 1];
            let y1 = self.efficiencies[i] * f(x1);
            let y2 = self.efficiencies[i + 1] * f(x2);

            // Area of trapezoid = (width) * (average height)
            sum += (x2 - x1) * (y1 + y2) / 2.0;
        }

        sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_valid_qe() {
        let wavelengths = vec![300.0, 400.0, 500.0, 600.0, 700.0, 800.0];
        let efficiencies = vec![0.0, 0.5, 0.8, 0.7, 0.3, 0.0];

        let qe = QuantumEfficiency::from_table(wavelengths, efficiencies).unwrap();

        // Test values at specific points
        assert_eq!(qe.at(300.0), 0.0);
        assert_eq!(qe.at(800.0), 0.0);
        assert_eq!(qe.at(500.0), 0.8);

        // Test interpolated values
        assert_relative_eq!(qe.at(450.0), 0.65, epsilon = 1e-5);
        assert_relative_eq!(qe.at(550.0), 0.75, epsilon = 1e-5);

        // Test values outside range
        assert_eq!(qe.at(200.0), 0.0);
        assert_eq!(qe.at(900.0), 0.0);
    }

    #[test]
    fn test_boundary_not_zero() {
        let wavelengths = vec![300.0, 400.0, 500.0];
        let efficiencies = vec![0.1, 0.5, 0.0]; // First value not zero

        let result = QuantumEfficiency::from_table(wavelengths, efficiencies);
        assert!(matches!(
            result,
            Err(QuantumEfficiencyError::BoundaryNotZero)
        ));
    }

    #[test]
    fn test_not_ascending() {
        let wavelengths = vec![300.0, 500.0, 400.0]; // Not in ascending order
        let efficiencies = vec![0.0, 0.5, 0.0];

        let result = QuantumEfficiency::from_table(wavelengths, efficiencies);
        assert!(matches!(result, Err(QuantumEfficiencyError::NotAscending)));
    }

    #[test]
    fn test_efficiency_out_of_range() {
        let wavelengths = vec![300.0, 400.0, 500.0];
        let efficiencies = vec![0.0, 1.2, 0.0]; // Value > 1.0

        let result = QuantumEfficiency::from_table(wavelengths, efficiencies);
        assert!(matches!(result, Err(QuantumEfficiencyError::OutOfRange)));
    }

    #[test]
    fn test_integrate() {
        let wavelengths = vec![300.0, 400.0, 500.0, 600.0];
        let efficiencies = vec![0.0, 0.5, 0.5, 0.0];

        let qe = QuantumEfficiency::from_table(wavelengths, efficiencies).unwrap();

        // Integrate with f(x) = 1.0
        // Area calculation:
        // First trapezoid: (400-300) * (0.0+0.5)/2 = 25
        // Second trapezoid: (500-400) * (0.5+0.5)/2 = 50
        // Third trapezoid: (600-500) * (0.5+0.0)/2 = 25
        // Total = 25 + 50 + 25 = 100
        let area = qe.integrate(|_| 1.0);
        assert_relative_eq!(area, 100.0, epsilon = 1e-5);
    }
}
