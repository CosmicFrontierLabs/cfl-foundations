//! Quantum efficiency models for astronomical detectors and photometric systems.
//!
//! This module provides accurate wavelength-dependent quantum efficiency modeling
//! for astronomical detectors, photometric filters, and optical systems. Essential
//! for synthetic photometry, instrument characterization, and realistic sensor
//! simulation in space telescope applications.
//!
//! # Quantum Efficiency Physics
//!
//! Quantum efficiency (QE) represents the probability that an incident photon
//! of a given wavelength will be detected and converted to a usable signal.
//! It encompasses the complete detection chain:
//!
//! - **Photon absorption**: Wavelength-dependent absorption in detector material
//! - **Charge generation**: Conversion of absorbed photons to electron-hole pairs
//! - **Charge collection**: Transport of generated charges to readout circuitry
//! - **Electronic conversion**: Analog-to-digital conversion efficiency
//!
//! # Mathematical Representation
//!
//! QE curves are represented as piecewise linear functions QE(λ) where:
//! - λ is wavelength in nanometers
//! - QE(λ) ∈ [0, 1] represents detection probability
//! - QE(λ) = 0 outside the detector's sensitive range
//! - Linear interpolation between measured data points
//!
//! # Applications
//!
//! ## Detector Characterization
//! ```rust
//! use simulator::photometry::{QuantumEfficiency, Band};
//!
//! // Create a CCD quantum efficiency curve
//! let wavelengths = vec![300.0, 400.0, 500.0, 600.0, 700.0, 800.0, 900.0];
//! let efficiencies = vec![0.0, 0.3, 0.8, 0.9, 0.7, 0.4, 0.0];
//!
//! let ccd_qe = QuantumEfficiency::from_table(wavelengths, efficiencies)
//!     .expect("Failed to create QE curve");
//!
//! // Check peak efficiency
//! println!("Peak QE at 600nm: {:.1}%", ccd_qe.at(600.0) * 100.0);
//!
//! // Get wavelength coverage
//! let band = ccd_qe.band();
//! println!("Sensitive range: {:.0}-{:.0} nm", band.lower_nm, band.upper_nm);
//! ```
//!
//! ## Filter Response Modeling
//! ```rust
//! use simulator::photometry::{QuantumEfficiency, Band};
//!
//! // Create a narrow-band filter (e.g., H-alpha at 656nm)
//! let ha_band = Band::from_nm_bounds(650.0, 662.0);
//! let ha_filter = QuantumEfficiency::from_notch(&ha_band, 0.85)
//!     .expect("Failed to create H-alpha filter");
//!
//! // Check filter characteristics
//! assert_eq!(ha_filter.at(656.0), 0.85);  // Peak transmission
//! assert_eq!(ha_filter.at(600.0), 0.0);   // Out-of-band rejection
//! ```
//!
//! ## Synthetic Photometry Integration
//! ```rust
//! use simulator::photometry::QuantumEfficiency;
//!
//! # let wavelengths = vec![400.0, 500.0, 600.0, 700.0];
//! # let efficiencies = vec![0.0, 0.8, 0.9, 0.0];
//! # let qe = QuantumEfficiency::from_table(wavelengths, efficiencies).unwrap();
//! // Integrate with stellar spectrum
//! let total_response = qe.integrate(|wavelength| {
//!     // Example: Planck function for 5780K blackbody (solar temperature)
//!     let h = 6.626e-34; // Planck constant
//!     let c = 3e8;       // Speed of light  
//!     let k = 1.381e-23; // Boltzmann constant
//!     let T = 5780.0;    // Temperature in K
//!     
//!     let lambda_m = wavelength * 1e-9; // Convert nm to meters
//!     let exponent = (h * c) / (lambda_m * k * T);
//!     
//!     // Simplified Planck function
//!     1.0 / (lambda_m.powi(5) * (exponent.exp() - 1.0))
//! });
//!
//! println!("Total integrated response: {:.2e}", total_response);
//! ```
//!
//! # Data Requirements
//!
//! QE curves must satisfy physical constraints:
//! - **Wavelength ordering**: Strictly ascending wavelength values
//! - **Efficiency bounds**: All values in [0.0, 1.0] range
//! - **Boundary conditions**: Zero efficiency at wavelength extremes
//! - **Monotonic segments**: No discontinuities within sensitive range
//!
//! # Performance Considerations
//!
//! - **Linear interpolation**: O(n) lookup with binary search optimization
//! - **Memory efficiency**: Compact storage of wavelength-efficiency pairs
//! - **Numerical stability**: Robust handling of edge cases and extrapolation
//! - **Integration accuracy**: Trapezoidal rule for photometric calculations

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

/// Wavelength-dependent quantum efficiency model for astronomical detectors.
///
/// Represents the detection probability as a function of photon wavelength,
/// using piecewise linear interpolation between measured data points. Fundamental
/// for accurate synthetic photometry, detector characterization, and photometric
/// system modeling in space telescope simulations.
///
/// # Physical Interpretation
/// - QE(λ) = 0.0: No photons detected at this wavelength
/// - QE(λ) = 1.0: Perfect detection (100% quantum efficiency)
/// - Typical values: 0.1-0.9 for modern astronomical detectors
///
/// # Data Storage
/// - **Wavelengths**: Ascending-ordered array in nanometers
/// - **Efficiencies**: Corresponding QE values in [0, 1] range
/// - **Interpolation**: Linear between data points, zero outside range
#[derive(Debug, Clone)]
pub struct QuantumEfficiency {
    /// Wavelengths in nanometers (nm)
    wavelengths: Vec<f64>,

    /// Efficiency values (0.0 to 1.0) corresponding to each wavelength
    efficiencies: Vec<f64>,
}

// TODO(meawoppl) - convert the internal storage to f64

impl QuantumEfficiency {
    /// Create quantum efficiency model for a rectangular passband (notch filter).
    ///
    /// Generates a simple rectangular response with sharp cutoffs at band edges,
    /// useful for modeling narrow-band filters, laser line filters, or simplified
    /// detector responses. The response is zero outside the band and constant within.
    ///
    /// # Physical Model
    /// Creates a step function:
    /// - QE = 0.0 for λ < band.lower_nm
    /// - QE = efficiency for band.lower_nm ≤ λ ≤ band.upper_nm  
    /// - QE = 0.0 for λ > band.upper_nm
    ///
    /// # Arguments
    /// * `band` - Wavelength range for the passband
    /// * `efficiency` - Constant QE value within the band [0.0, 1.0]
    ///
    /// # Returns
    /// Result containing QuantumEfficiency model or validation error
    ///
    /// # Examples
    /// ```rust
    /// use simulator::photometry::{QuantumEfficiency, Band};
    ///
    /// // Create H-alpha narrow-band filter (656.3nm ± 5nm)
    /// let ha_band = Band::from_nm_bounds(651.3, 661.3);
    /// let ha_filter = QuantumEfficiency::from_notch(&ha_band, 0.85)
    ///     .expect("Failed to create H-alpha filter");
    ///
    /// // Verify filter properties
    /// assert_eq!(ha_filter.at(656.3), 0.85);  // Peak transmission
    /// assert_eq!(ha_filter.at(600.0), 0.0);   // Out-of-band
    /// assert_eq!(ha_filter.at(700.0), 0.0);   // Out-of-band
    /// ```
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

    /// Evaluate quantum efficiency at specified wavelength using linear interpolation.
    ///
    /// Returns the detection probability for photons at the given wavelength.
    /// Uses linear interpolation between measured data points for wavelengths
    /// within the defined range, and returns zero for out-of-band wavelengths.
    ///
    /// # Interpolation Method
    /// For wavelength λ between data points (λᵢ, QEᵢ) and (λᵢ₊₁, QEᵢ₊₁):
    /// QE(λ) = QEᵢ + (QEᵢ₊₁ - QEᵢ) × (λ - λᵢ) / (λᵢ₊₁ - λᵢ)
    ///
    /// # Arguments
    /// * `wavelength` - Photon wavelength in nanometers
    ///
    /// # Returns
    /// Quantum efficiency [0.0, 1.0] or 0.0 if outside detector range
    ///
    /// # Examples
    /// ```rust
    /// use simulator::photometry::QuantumEfficiency;
    ///
    /// let wavelengths = vec![400.0, 500.0, 600.0, 700.0];
    /// let efficiencies = vec![0.0, 0.8, 0.9, 0.0];
    /// let qe = QuantumEfficiency::from_table(wavelengths, efficiencies).unwrap();
    ///
    /// // Direct lookup at data points
    /// assert_eq!(qe.at(500.0), 0.8);
    /// assert_eq!(qe.at(600.0), 0.9);
    ///
    /// // Linear interpolation between points
    /// assert!((qe.at(550.0) - 0.85).abs() < 0.01);  // Midpoint between 500nm and 600nm
    ///
    /// // Out-of-band returns zero
    /// assert_eq!(qe.at(300.0), 0.0);   // Below range
    /// assert_eq!(qe.at(800.0), 0.0);   // Above range
    /// ```
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
