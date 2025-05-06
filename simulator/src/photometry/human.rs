//! Human vision quantum efficiency models
//!
//! This module provides quantum efficiency models for human vision,
//! representing the spectral sensitivity of human eye photoreceptors.

use super::quantum_efficiency::QuantumEfficiency;
use super::spectrum::Band;

/// Human eye photoreceptor types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanPhotoreceptor {
    /// Red channel (L-cone) photoreceptor
    Red,
    /// Green-Red channel (hybrid M-cone) photoreceptor
    GreenRed,
    /// Green-Blue channel (hybrid M-cone) photoreceptor
    GreenBlue,
    /// Blue channel (S-cone) photoreceptor
    Blue,
}

/// Functions for creating and working with human vision quantum efficiency curves
pub struct HumanVision {}

impl HumanVision {
    /// Standard wavelength vector for human vision QE curves (nm)
    /// Ranges from 0nm, then 350nm to 1050nm at 50nm intervals, ending with 1100nm
    fn standard_wavelengths() -> Vec<f64> {
        vec![
            0.0, 350.0, 400.0, 450.0, 500.0, 550.0, 600.0, 650.0, 700.0, 750.0, 800.0, 850.0,
            900.0, 950.0, 1000.0, 1050.0, 1100.0,
        ]
    }

    /// Get quantum efficiency for the red channel (L-cone) photoreceptors
    ///
    /// # Returns
    ///
    /// A `QuantumEfficiency` instance representing the red channel
    pub fn red() -> QuantumEfficiency {
        let wavelengths = Self::standard_wavelengths();
        let efficiencies = vec![
            0.0, 0.005, 0.025, 0.035, 0.06, 0.21, 0.32, 0.26, 0.21, 0.19, 0.16, 0.12, 0.07, 0.04,
            0.02, 0.01, 0.0,
        ];

        QuantumEfficiency::from_table(wavelengths, efficiencies)
            .expect("Red QE curve should be valid")
    }

    /// Get quantum efficiency for the blue channel (S-cone) photoreceptors
    ///
    /// # Returns
    ///
    /// A `QuantumEfficiency` instance representing the blue channel
    pub fn blue() -> QuantumEfficiency {
        let wavelengths = Self::standard_wavelengths();
        let efficiencies = vec![
            0.0, 0.01, 0.18, 0.33, 0.18, 0.05, 0.03, 0.025, 0.035, 0.05, 0.15, 0.10, 0.07, 0.04,
            0.02, 0.01, 0.0,
        ];

        QuantumEfficiency::from_table(wavelengths, efficiencies)
            .expect("Blue QE curve should be valid")
    }

    /// Get quantum efficiency for the green-red channel (hybrid M-cone) photoreceptors
    ///
    /// # Returns
    ///
    /// A `QuantumEfficiency` instance representing the green-red channel
    pub fn green_red() -> QuantumEfficiency {
        let wavelengths = Self::standard_wavelengths();
        let efficiencies = vec![
            0.0, 0.008, 0.035, 0.05, 0.12, 0.19, 0.10, 0.08, 0.09, 0.10, 0.15, 0.12, 0.07, 0.04,
            0.02, 0.01, 0.0,
        ];

        QuantumEfficiency::from_table(wavelengths, efficiencies)
            .expect("Green-Red QE curve should be valid")
    }

    /// Get quantum efficiency for the green-blue channel (hybrid M-cone) photoreceptors
    ///
    /// # Returns
    ///
    /// A `QuantumEfficiency` instance representing the green-blue channel
    pub fn green_blue() -> QuantumEfficiency {
        let wavelengths = Self::standard_wavelengths();
        let efficiencies = vec![
            0.0, 0.008, 0.06, 0.24, 0.40, 0.15, 0.06, 0.05, 0.08, 0.10, 0.15, 0.12, 0.07, 0.04,
            0.02, 0.01, 0.0,
        ];

        QuantumEfficiency::from_table(wavelengths, efficiencies)
            .expect("Green-Blue QE curve should be valid")
    }

    /// Get quantum efficiency for a specific human photoreceptor type
    ///
    /// # Arguments
    ///
    /// * `receptor` - The photoreceptor type to get the QE for
    ///
    /// # Returns
    ///
    /// A `QuantumEfficiency` instance for the specified photoreceptor
    pub fn for_receptor(receptor: HumanPhotoreceptor) -> QuantumEfficiency {
        match receptor {
            HumanPhotoreceptor::Red => Self::red(),
            HumanPhotoreceptor::Blue => Self::blue(),
            HumanPhotoreceptor::GreenRed => Self::green_red(),
            HumanPhotoreceptor::GreenBlue => Self::green_blue(),
        }
    }

    /// Get the visible spectrum band (approximately 350-750nm)
    ///
    /// # Returns
    ///
    /// A `Band` representing the human visible spectrum
    pub fn visible_band() -> Band {
        Band::from_nm_bounds(350.0, 750.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_human_qe_curves_valid() {
        // Test that all human QE curves can be created without errors
        let red_qe = HumanVision::red();
        let blue_qe = HumanVision::blue();
        let green_red_qe = HumanVision::green_red();
        let green_blue_qe = HumanVision::green_blue();

        // Check that the curves span from 0 to 1100nm
        assert_eq!(red_qe.band().lower_nm, 0.0);
        assert_eq!(red_qe.band().upper_nm, 1100.0);

        assert_eq!(blue_qe.band().lower_nm, 0.0);
        assert_eq!(blue_qe.band().upper_nm, 1100.0);

        assert_eq!(green_red_qe.band().lower_nm, 0.0);
        assert_eq!(green_red_qe.band().upper_nm, 1100.0);

        assert_eq!(green_blue_qe.band().lower_nm, 0.0);
        assert_eq!(green_blue_qe.band().upper_nm, 1100.0);
    }

    #[test]
    fn test_peak_wavelengths() {
        // Check that the peak sensitivity occurs at expected wavelengths
        let red_qe = HumanVision::red();
        let blue_qe = HumanVision::blue();
        let green_red_qe = HumanVision::green_red();
        let green_blue_qe = HumanVision::green_blue();

        // Red peak should be around 600nm
        assert_relative_eq!(red_qe.at(600.0), 0.32);

        // Blue peak should be around 450nm
        assert_relative_eq!(blue_qe.at(450.0), 0.33);

        // Green-Red peak should be around 550nm
        assert_relative_eq!(green_red_qe.at(550.0), 0.19);

        // Green-Blue peak should be around 500nm
        assert_relative_eq!(green_blue_qe.at(500.0), 0.40);
    }

    #[test]
    fn test_receptor_lookup() {
        let red_direct = HumanVision::red();
        let red_lookup = HumanVision::for_receptor(HumanPhotoreceptor::Red);

        // Both methods should give identical curves
        assert_relative_eq!(red_direct.at(500.0), red_lookup.at(500.0));
        assert_relative_eq!(red_direct.at(600.0), red_lookup.at(600.0));
    }

    #[test]
    fn test_visible_band() {
        let band = HumanVision::visible_band();

        // Check band bounds
        assert_eq!(band.lower_nm, 350.0);
        assert_eq!(band.upper_nm, 750.0);
    }

    #[test]
    fn test_qe_interpolation() {
        let red_qe = HumanVision::red();

        // Test interpolation between known points
        let expected_425nm = (0.025 + 0.035) / 2.0; // Average of 400nm (0.025) and 450nm (0.035)
        assert_relative_eq!(red_qe.at(425.0), expected_425nm, epsilon = 1e-5);
    }
}
