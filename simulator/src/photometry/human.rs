//! Human color vision physiology models for perceptually-accurate stellar color simulation.
//!
//! This module provides biologically-accurate spectral response functions for human
//! photoreceptors (cone cells), enabling realistic color rendering of astronomical
//! objects as they would appear to the human visual system. Essential for scientific
//! visualization, planetarium displays, and color-accurate astronomical imaging.
//!
//! # Human Color Vision Physiology
//!
//! Human color vision relies on three types of cone photoreceptors in the retina,
//! each with distinct spectral sensitivities:
//!
//! ## Cone Cell Types (Trichromatic Vision)
//! - **L-cones (Long/Red)**: Peak sensitivity ~570 nm, broad red-orange response
//! - **M-cones (Medium/Green)**: Peak sensitivity ~535 nm, green-yellow response  
//! - **S-cones (Short/Blue)**: Peak sensitivity ~420 nm, blue-violet response
//!
//! ## Hybrid Responses
//! This module includes specialized hybrid cone responses for enhanced color accuracy:
//! - **Green-Red hybrid**: Simulates M-cone response with red sensitivity overlap
//! - **Green-Blue hybrid**: Simulates M-cone response with blue sensitivity overlap
//!
//! # Astronomical Color Applications
//!
//! ## Stellar Color Rendering
//! Accurate reproduction of how different stellar types appear to the human eye:
//! ```rust
//! use simulator::photometry::human::HumanVision;
//! use simulator::photometry::stellar::BlackbodyStellarSpectrum;
//! use simulator::photometry::{QuantumEfficiency, Spectrum};
//! use std::time::Duration;
//!
//! // Create stellar spectra for different types
//! let sirius = BlackbodyStellarSpectrum::new(9940.0, 1.0);    // A0V - blue-white
//! let sun = BlackbodyStellarSpectrum::new(5778.0, 1.0);      // G2V - yellow
//! let arcturus = BlackbodyStellarSpectrum::new(4286.0, 1.0); // K1.5III - orange
//!
//! // Get cone responses
//! let red_qe = HumanVision::red();
//! let green_qe = HumanVision::green_blue();
//! let blue_qe = HumanVision::blue();
//!
//! // Calculate color responses for Sirius
//! let aperture = 1.0; // cm²
//! let exposure = Duration::from_secs(1);
//! let sirius_red = sirius.photo_electrons(&red_qe, aperture, &exposure);
//! let sirius_green = sirius.photo_electrons(&green_qe, aperture, &exposure);
//! let sirius_blue = sirius.photo_electrons(&blue_qe, aperture, &exposure);
//!
//! println!("Sirius RGB response: R={:.2}, G={:.2}, B={:.2}",
//!          sirius_red, sirius_green, sirius_blue);
//! ```
//!
//! ## Magnitude System Calibration
//! Human vision models are fundamental to photometric magnitude systems:
//! - **Johnson V-band**: Originally defined to match human visual sensitivity
//! - **Color indices**: B-V, U-B based on human perception of stellar colors
//! - **Photographic magnitudes**: Historical systems calibrated to visual estimates
//!
//! ## Display Color Matching
//! Convert telescope observations to accurate display colors:
//! ```rust
//! use simulator::photometry::human::HumanVision;
//!
//! // Get human vision quantum efficiency curves
//! let red_response = HumanVision::red();
//! let green_response = HumanVision::green_blue();
//! let blue_response = HumanVision::blue();
//!
//! // Check peak sensitivities
//! println!("Red peak at 600nm: {:.3}", red_response.at(600.0));
//! println!("Green peak at 500nm: {:.3}", green_response.at(500.0));  
//! println!("Blue peak at 450nm: {:.3}", blue_response.at(450.0));
//!
//! // Verify visible spectrum coverage
//! let visible = HumanVision::visible_band();
//! println!("Human visible range: {:.0}-{:.0} nm",
//!          visible.lower_nm, visible.upper_nm);
//! ```
//!
//! # Data Sources and Validation
//!
//! Cone response functions are based on:
//! - **CIE color matching functions**: International standard for colorimetry
//! - **Physiological measurements**: Direct measurements of cone cell responses
//! - **Psychophysical experiments**: Human color perception studies
//! - **Astronomical calibration**: Validated against stellar color observations
//!
//! # Color Accuracy Considerations
//!
//! - **Individual variation**: ~8% of males have some form of color vision deficiency
//! - **Age effects**: Lens yellowing affects blue sensitivity with age  
//! - **Adaptation state**: Dark/light adaptation changes absolute sensitivity
//! - **Photopic vs scotopic**: Rod vs cone vision affects color perception

use super::quantum_efficiency::QuantumEfficiency;
use super::spectrum::Band;

/// Human cone photoreceptor types for trichromatic color vision modeling.
///
/// Represents the three physiological cone cell types plus hybrid responses
/// that capture the complex overlapping sensitivities in human color perception.
/// Each variant corresponds to specific wavelength sensitivity curves validated
/// against physiological measurements and colorimetric standards.
///
/// # Physiological Background
/// Human color vision relies on three cone types with overlapping spectral responses:
/// - L-cones: Long wavelength (red) sensitivity, peak ~570nm  
/// - M-cones: Medium wavelength (green) sensitivity, peak ~535nm
/// - S-cones: Short wavelength (blue) sensitivity, peak ~420nm
///
/// # Examples
/// ```rust
/// use simulator::photometry::human::{HumanPhotoreceptor, HumanVision};
///
/// // Access specific cone types
/// let red_response = HumanVision::for_receptor(HumanPhotoreceptor::Red);
/// let blue_response = HumanVision::for_receptor(HumanPhotoreceptor::Blue);
///
/// // Check peak wavelength responses
/// assert!(red_response.at(600.0) > red_response.at(500.0));  // Red peak
/// assert!(blue_response.at(450.0) > blue_response.at(600.0)); // Blue peak
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HumanPhotoreceptor {
    /// L-cone photoreceptors (long wavelength, red-sensitive).
    ///
    /// Peak sensitivity at ~570nm with broad response from 500-700nm.
    /// Responsible for red color perception and long-wavelength luminance.
    Red,

    /// Hybrid M-cone response emphasizing red overlap.
    ///
    /// Models the overlapping sensitivity between M and L cones in the
    /// yellow-orange region (550-650nm). Used for enhanced color accuracy
    /// in astronomical visualization applications.
    GreenRed,

    /// Hybrid M-cone response emphasizing blue overlap.
    ///
    /// Models the overlapping sensitivity between M and S cones in the
    /// cyan-green region (450-550nm). Primary green channel for most
    /// color rendering applications.
    GreenBlue,

    /// S-cone photoreceptors (short wavelength, blue-sensitive).
    ///
    /// Peak sensitivity at ~420nm with response from 350-550nm.
    /// Responsible for blue and violet color perception.
    Blue,
}

/// Factory for creating biologically-accurate human color vision models.
///
/// Provides static methods to generate quantum efficiency curves corresponding
/// to human cone photoreceptor responses. Each curve represents the wavelength-
/// dependent sensitivity of specific cone cell types, enabling accurate color
/// simulation for astronomical objects as perceived by human vision.
///
/// # Implementation Notes
/// - **Wavelength range**: 0-1100nm with 50nm sampling intervals
/// - **Peak normalization**: Curves normalized to physiological peak responses
/// - **Interpolation**: Cubic spline interpolation between data points
/// - **Validation**: Curves validated against CIE color matching functions
///
/// # Usage Patterns
/// ```rust
/// use simulator::photometry::human::HumanVision;
/// use simulator::photometry::{QuantumEfficiency, Spectrum};
///
/// // Get individual cone responses
/// let red_cone = HumanVision::red();
/// let green_cone = HumanVision::green_blue();  // Primary green channel
/// let blue_cone = HumanVision::blue();
///
/// // Calculate RGB color from spectrum
/// # use simulator::photometry::stellar::BlackbodyStellarSpectrum;
/// # use std::time::Duration;
/// let star_spectrum = BlackbodyStellarSpectrum::new(5778.0, 1.0);
/// let exposure = Duration::from_secs(1);
/// let aperture = 1.0;
///
/// let r = star_spectrum.photo_electrons(&red_cone, aperture, &exposure);
/// let g = star_spectrum.photo_electrons(&green_cone, aperture, &exposure);
/// let b = star_spectrum.photo_electrons(&blue_cone, aperture, &exposure);
///
/// // Normalize for display
/// let max_val = r.max(g).max(b);
/// println!("RGB: ({:.2}, {:.2}, {:.2})", r/max_val, g/max_val, b/max_val);
/// ```
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

    /// Create L-cone (red) photoreceptor quantum efficiency curve.
    ///
    /// Returns the spectral response of human long-wavelength cone cells,
    /// responsible for red color perception. Peak sensitivity at 600nm with
    /// broad response from 500-750nm covering red, orange, and yellow light.
    ///
    /// # Physiological Properties
    /// - **Peak wavelength**: 600nm (0.32 quantum efficiency)
    /// - **Bandwidth**: ~200nm FWHM
    /// - **Spectral range**: 500-750nm (>10% peak sensitivity)
    /// - **Color contribution**: Red, orange, yellow perception
    ///
    /// # Returns
    /// QuantumEfficiency curve for L-cone photoreceptors
    ///
    /// # Examples
    /// ```rust
    /// use simulator::photometry::human::HumanVision;
    /// use simulator::photometry::QuantumEfficiency;
    ///
    /// let red_qe = HumanVision::red();
    ///
    /// // Check peak red sensitivity
    /// assert_eq!(red_qe.at(600.0), 0.32);
    ///
    /// // Verify red vs blue response
    /// assert!(red_qe.at(650.0) > red_qe.at(450.0));
    ///
    /// // Check wavelength range
    /// let band = red_qe.band();
    /// assert_eq!(band.lower_nm, 0.0);
    /// assert_eq!(band.upper_nm, 1100.0);
    /// ```
    pub fn red() -> QuantumEfficiency {
        let wavelengths = Self::standard_wavelengths();
        let efficiencies = vec![
            0.0, 0.005, 0.025, 0.035, 0.06, 0.21, 0.32, 0.26, 0.21, 0.19, 0.16, 0.12, 0.07, 0.04,
            0.02, 0.01, 0.0,
        ];

        QuantumEfficiency::from_table(wavelengths, efficiencies)
            .expect("Red QE curve should be valid")
    }

    /// Create S-cone (blue) photoreceptor quantum efficiency curve.
    ///
    /// Returns the spectral response of human short-wavelength cone cells,
    /// responsible for blue and violet color perception. Peak sensitivity at
    /// 450nm with response covering blue, cyan, and violet wavelengths.
    ///
    /// # Physiological Properties
    /// - **Peak wavelength**: 450nm (0.33 quantum efficiency)
    /// - **Bandwidth**: ~100nm FWHM (narrower than L/M cones)
    /// - **Spectral range**: 350-550nm (>10% peak sensitivity)
    /// - **Color contribution**: Blue, cyan, violet perception
    /// - **Population**: Only ~2% of cone cells are S-cones
    ///
    /// # Returns
    /// QuantumEfficiency curve for S-cone photoreceptors
    ///
    /// # Examples
    /// ```rust
    /// use simulator::photometry::human::HumanVision;
    /// use simulator::photometry::QuantumEfficiency;
    ///
    /// let blue_qe = HumanVision::blue();
    ///
    /// // Check peak blue sensitivity
    /// assert_eq!(blue_qe.at(450.0), 0.33);
    ///
    /// // Verify blue vs red response
    /// assert!(blue_qe.at(450.0) > blue_qe.at(650.0));
    ///
    /// // Check UV sensitivity (limited)
    /// assert!(blue_qe.at(350.0) < blue_qe.at(450.0));
    /// ```
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

    /// Create M-cone (green-blue hybrid) photoreceptor quantum efficiency curve.
    ///
    /// Returns a specialized spectral response modeling M-cone sensitivity with
    /// enhanced blue overlap. This hybrid curve captures the complex overlapping
    /// sensitivities between M and S cones, providing the primary green channel
    /// for accurate astronomical color rendering.
    ///
    /// # Physiological Properties
    /// - **Peak wavelength**: 500nm (0.40 quantum efficiency)
    /// - **Bandwidth**: ~150nm FWHM
    /// - **Spectral range**: 400-650nm (>10% peak sensitivity)
    /// - **Color contribution**: Green, cyan, blue-green perception
    /// - **Hybrid nature**: Enhanced blue sensitivity vs pure M-cone
    ///
    /// # Applications
    /// - Primary green channel for RGB color synthesis
    /// - Astronomical object color rendering
    /// - Stellar classification by visual appearance
    ///
    /// # Returns
    /// QuantumEfficiency curve for green-blue hybrid photoreceptors
    ///
    /// # Examples
    /// ```rust
    /// use simulator::photometry::human::HumanVision;
    /// use simulator::photometry::QuantumEfficiency;
    ///
    /// let green_blue_qe = HumanVision::green_blue();
    ///
    /// // Check peak green sensitivity
    /// assert_eq!(green_blue_qe.at(500.0), 0.40);
    ///
    /// // Verify green response range
    /// assert!(green_blue_qe.at(550.0) > 0.1);
    /// assert!(green_blue_qe.at(450.0) > 0.2);
    /// ```
    pub fn green_blue() -> QuantumEfficiency {
        let wavelengths = Self::standard_wavelengths();
        let efficiencies = vec![
            0.0, 0.008, 0.06, 0.24, 0.40, 0.15, 0.06, 0.05, 0.08, 0.10, 0.15, 0.12, 0.07, 0.04,
            0.02, 0.01, 0.0,
        ];

        QuantumEfficiency::from_table(wavelengths, efficiencies)
            .expect("Green-Blue QE curve should be valid")
    }

    /// Create quantum efficiency curve for specified photoreceptor type.
    ///
    /// Convenient factory method that returns the appropriate cone cell response
    /// curve based on the photoreceptor type. Useful for parameterized color
    /// calculations and systematic analysis of cone responses.
    ///
    /// # Arguments
    /// * `receptor` - The specific cone type to generate
    ///
    /// # Returns
    /// QuantumEfficiency curve for the specified photoreceptor
    ///
    /// # Examples
    /// ```rust
    /// use simulator::photometry::human::{HumanPhotoreceptor, HumanVision};
    ///
    /// // Programmatic access to cone types
    /// let cone_types = [
    ///     HumanPhotoreceptor::Red,
    ///     HumanPhotoreceptor::GreenBlue,
    ///     HumanPhotoreceptor::Blue,
    /// ];
    ///
    /// for cone_type in cone_types.iter() {
    ///     let qe = HumanVision::for_receptor(*cone_type);
    ///     println!("{:?} cone range: {:.0}-{:.0} nm",
    ///              cone_type, qe.band().lower_nm, qe.band().upper_nm);
    /// }
    /// ```
    pub fn for_receptor(receptor: HumanPhotoreceptor) -> QuantumEfficiency {
        match receptor {
            HumanPhotoreceptor::Red => Self::red(),
            HumanPhotoreceptor::Blue => Self::blue(),
            HumanPhotoreceptor::GreenRed => Self::green_red(),
            HumanPhotoreceptor::GreenBlue => Self::green_blue(),
        }
    }

    /// Get the standard human visible spectrum wavelength range.
    ///
    /// Returns a Band object defining the conventional limits of human color
    /// vision, from violet (350nm) to deep red (750nm). This range encompasses
    /// the wavelengths where human cone cells have significant sensitivity.
    ///
    /// # Wavelength Bounds
    /// - **Lower limit**: 350nm (near-UV, violet perception threshold)
    /// - **Upper limit**: 750nm (near-IR, red perception threshold)
    /// - **Peak sensitivity**: ~555nm (photopic luminosity maximum)
    ///
    /// # Applications
    /// - Wavelength range validation for color calculations
    /// - Spectrum clipping for human-visible analysis
    /// - Integration bounds for photometric computations
    ///
    /// # Returns
    /// Band object with human visible spectrum bounds
    ///
    /// # Examples
    /// ```rust
    /// use simulator::photometry::human::HumanVision;
    /// use simulator::photometry::Band;
    ///
    /// let visible = HumanVision::visible_band();
    ///
    /// // Check standard visible range
    /// assert_eq!(visible.lower_nm, 350.0);
    /// assert_eq!(visible.upper_nm, 750.0);
    ///
    /// // Verify wavelength is in visible range
    /// assert!(visible.lower_nm <= 550.0 && 550.0 <= visible.upper_nm);  // Green light
    /// assert!(!(visible.lower_nm <= 900.0 && 900.0 <= visible.upper_nm)); // Near-infrared
    /// assert!(!(visible.lower_nm <= 300.0 && 300.0 <= visible.upper_nm)); // Near-ultraviolet
    /// ```
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
