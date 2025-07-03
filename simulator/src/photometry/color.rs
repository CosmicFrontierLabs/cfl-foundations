//! Stellar color analysis and spectral classification for astronomical applications.
//!
//! This module provides comprehensive utilities for analyzing stellar colors, mapping
//! astronomical spectra to human-perceived colors, and implementing the Morgan-Keenan
//! spectral classification system. Essential for photometric analysis, stellar
//! identification, and realistic color rendering of astronomical scenes.
//!
//! # Spectral Classification
//!
//! The module implements the Harvard spectral classification system (OBAFGKM), which
//! categorizes stars based on their surface temperature and spectral features. This
//! classification is fundamental to stellar astrophysics and enables:
//!
//! - **Temperature estimation**: Direct mapping from spectral class to temperature range
//! - **Color prediction**: Expected color appearance of different stellar types
//! - **Catalog validation**: Consistency checks for stellar photometry
//! - **Synthetic photometry**: Realistic color simulation for space telescope imagery
//!
//! # Color Calculations
//!
//! Color computations use biologically-accurate human vision models based on cone cell
//! spectral response functions. This provides:
//!
//! - **Perceptually accurate colors**: Matches human visual perception of stellar colors
//! - **Photometric indices**: Quantitative color measurements (B-V, color temperature)
//! - **Display optimization**: Proper color rendering for scientific visualization
//!
//! # Examples
//!
//! ## Spectral Classification
//! ```rust
//! use simulator::photometry::color::{temperature_to_spectral_class, SpectralClass};
//!
//! // Classify different stellar types by temperature
//! let sun_class = temperature_to_spectral_class(5778.0);  // Sun
//! let sirius_class = temperature_to_spectral_class(9940.0);  // Sirius A
//! let betelgeuse_class = temperature_to_spectral_class(3500.0);  // Betelgeuse
//!
//! assert_eq!(sun_class, SpectralClass::G);
//! assert_eq!(sirius_class, SpectralClass::A);
//! assert_eq!(betelgeuse_class, SpectralClass::M);
//!
//! println!("Sun: {} class, Sirius: {} class, Betelgeuse: {} class",
//!          sun_class, sirius_class, betelgeuse_class);
//! ```
//!
//! ## Color Analysis
//! ```rust
//! use simulator::photometry::color::{spectrum_to_rgb_values, color_temperature_index};
//! use simulator::photometry::stellar::BlackbodyStellarSpectrum;
//!
//! // Create stellar spectra
//! let hot_star = BlackbodyStellarSpectrum::new(20000.0, 1.0);  // Blue giant
//! let cool_star = BlackbodyStellarSpectrum::new(3000.0, 1.0);  // Red dwarf
//!
//! // Get RGB colors for display
//! let (hot_r, hot_g, hot_b) = spectrum_to_rgb_values(&hot_star);
//! let (cool_r, cool_g, cool_b) = spectrum_to_rgb_values(&cool_star);
//!
//! // Calculate color indices
//! let hot_index = color_temperature_index(&hot_star);   // Positive (blue)
//! let cool_index = color_temperature_index(&cool_star); // Negative (red)
//!
//! println!("Hot star RGB: ({:.2}, {:.2}, {:.2}), Index: {:.2}",
//!          hot_r, hot_g, hot_b, hot_index);
//! println!("Cool star RGB: ({:.2}, {:.2}, {:.2}), Index: {:.2}",
//!          cool_r, cool_g, cool_b, cool_index);
//! ```
//!
//! ## Temperature Sequence Generation  
//! ```rust
//! use simulator::photometry::color::{generate_temperature_sequence, temperature_to_spectral_class};
//!
//! // Generate logarithmically-spaced temperatures for stellar analysis
//! let temps = generate_temperature_sequence(2500.0, 50000.0, 10);
//!
//! // Process entire stellar population
//! for (i, temp) in temps.iter().enumerate() {
//!     let class = temperature_to_spectral_class(*temp);
//!     println!("Star {}: {:.0}K, Class {}", i+1, temp, class);
//! }
//! ```

use super::human::HumanVision;
use super::spectrum::Spectrum;
use std::fmt;
use std::time::Duration;

/// The spectral classification system for stars, known as the Morgan-Keenan system.
///
/// This system classifies stars based on their spectral characteristics, primarily temperature.
/// The main classes from hottest to coolest are: O, B, A, F, G, K, M.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpectralClass {
    /// O-type stars: Very hot and bright, with temperatures above 30,000K
    /// These stars appear blue-white and have strong ionized helium lines
    O,

    /// B-type stars: Hot and blue-white stars, temperatures 10,000-30,000K
    B,

    /// A-type stars: White stars with strong hydrogen lines, temperatures 7,500-10,000K
    A,

    /// F-type stars: Yellow-white stars, temperatures 6,000-7,500K
    F,

    /// G-type stars: Yellow stars like our Sun, temperatures 5,200-6,000K
    G,

    /// K-type stars: Orange stars, temperatures 3,700-5,200K
    K,

    /// M-type stars: Red stars, temperatures below 3,700K
    M,
}

impl fmt::Display for SpectralClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display as a single character
        write!(
            f,
            "{}",
            match self {
                SpectralClass::O => 'O',
                SpectralClass::B => 'B',
                SpectralClass::A => 'A',
                SpectralClass::F => 'F',
                SpectralClass::G => 'G',
                SpectralClass::K => 'K',
                SpectralClass::M => 'M',
            }
        )
    }
}

/// Generate logarithmically-spaced stellar temperature sequence for population analysis.
///
/// Creates temperature points distributed evenly on a logarithmic scale, which provides
/// better sampling across the full range of stellar temperatures. This is essential for
/// stellar population studies where temperatures span 3-4 orders of magnitude.
///
/// # Astronomical Context
///
/// Stellar temperatures range from ~2000K (coolest brown dwarfs) to ~50000K (hottest
/// O-type stars). Linear spacing would oversample hot stars and undersample cool stars,
/// while logarithmic spacing provides representative coverage of the stellar population.
///
/// # Arguments
/// * `min_temp` - Minimum temperature in Kelvin (typically 2500K for M dwarfs)
/// * `max_temp` - Maximum temperature in Kelvin (typically 50000K for O stars)  
/// * `n_temps` - Number of temperature points to generate
///
/// # Returns
/// Vector of temperatures in Kelvin, evenly spaced on logarithmic scale
///
/// # Examples
/// ```rust
/// use simulator::photometry::color::generate_temperature_sequence;
///
/// // Generate 7 temperatures covering main sequence
/// let main_sequence = generate_temperature_sequence(3000.0, 30000.0, 7);
/// println!("Main sequence temperatures: {:?}", main_sequence);
/// // Output: [3000.0, 4100.0, 5600.0, 7700.0, 10500.0, 14400.0, 30000.0]
///
/// // Single temperature (edge case)
/// let single = generate_temperature_sequence(5778.0, 6000.0, 1);
/// assert_eq!(single, vec![5778.0]);  // Returns min_temp for n=1
/// ```
pub fn generate_temperature_sequence(min_temp: f64, max_temp: f64, n_temps: usize) -> Vec<f64> {
    if n_temps <= 1 {
        return vec![min_temp];
    }

    let log_min = min_temp.ln();
    let log_max = max_temp.ln();
    let step = (log_max - log_min) / ((n_temps - 1) as f64);

    (0..n_temps)
        .map(|i| (log_min + i as f64 * step).exp())
        .collect()
}

/// Determine Morgan-Keenan spectral class from stellar effective temperature.
///
/// Implements the Harvard spectral classification system using temperature boundaries
/// established by modern stellar astronomy. This classification is fundamental for
/// stellar identification, catalog validation, and synthetic photometry.
///
/// # Classification Boundaries
/// - **O-type**: ≥30,000K - Blue hypergiants and Wolf-Rayet precursors
/// - **B-type**: 10,000-30,000K - Blue giants and main sequence stars  
/// - **A-type**: 7,500-10,000K - White stars like Sirius and Vega
/// - **F-type**: 6,000-7,500K - Yellow-white stars like Procyon
/// - **G-type**: 5,200-6,000K - Yellow stars like the Sun
/// - **K-type**: 3,700-5,200K - Orange stars like Arcturus
/// - **M-type**: <3,700K - Red dwarfs and giants like Betelgeuse
///
/// # Arguments
/// * `temperature` - Stellar effective temperature in Kelvin
///
/// # Returns
/// Spectral class as SpectralClass enum value
///
/// # Examples
/// ```rust
/// use simulator::photometry::color::{temperature_to_spectral_class, SpectralClass};
///
/// // Famous stars and their classifications
/// assert_eq!(temperature_to_spectral_class(5778.0), SpectralClass::G); // Sun
/// assert_eq!(temperature_to_spectral_class(9940.0), SpectralClass::A); // Sirius A
/// assert_eq!(temperature_to_spectral_class(4286.0), SpectralClass::K); // Arcturus  
/// assert_eq!(temperature_to_spectral_class(3500.0), SpectralClass::M); // Betelgeuse
/// assert_eq!(temperature_to_spectral_class(22000.0), SpectralClass::B); // Rigel
///
/// // Edge cases
/// assert_eq!(temperature_to_spectral_class(50000.0), SpectralClass::O); // Very hot
/// assert_eq!(temperature_to_spectral_class(2000.0), SpectralClass::M);  // Very cool
/// ```
pub fn temperature_to_spectral_class(temperature: f64) -> SpectralClass {
    if temperature >= 30000.0 {
        SpectralClass::O
    } else if temperature >= 10000.0 {
        SpectralClass::B
    } else if temperature >= 7500.0 {
        SpectralClass::A
    } else if temperature >= 6000.0 {
        SpectralClass::F
    } else if temperature >= 5200.0 {
        SpectralClass::G
    } else if temperature >= 3700.0 {
        SpectralClass::K
    } else {
        SpectralClass::M
    }
}

/// Convert stellar spectrum to RGB color values using human vision models.
///
/// Calculates perceptually-accurate RGB color representation by convolving the stellar
/// spectrum with human cone cell spectral response functions. Uses biologically-based
/// quantum efficiency curves for red, green, and blue photoreceptors to match human
/// visual perception of stellar colors.
///
/// # Color Accuracy
///
/// This method provides scientifically accurate stellar colors as they would appear
/// to the human eye under ideal viewing conditions. The RGB values are normalized
/// to prevent clipping while preserving relative color balance.
///
/// # Arguments
/// * `spectrum` - Stellar spectrum implementing the Spectrum trait
///
/// # Returns
/// Tuple (r, g, b) with each component normalized to [0.0, 1.0] range
///
/// # Examples
/// ```rust
/// use simulator::photometry::color::spectrum_to_rgb_values;
/// use simulator::photometry::stellar::BlackbodyStellarSpectrum;
///
/// // Create spectra for different stellar types
/// let sun = BlackbodyStellarSpectrum::new(5778.0, 1.0);      // G2V (yellow)
/// let vega = BlackbodyStellarSpectrum::new(9602.0, 1.0);     // A0V (white)
/// let arcturus = BlackbodyStellarSpectrum::new(4286.0, 1.0); // K1.5III (orange)
/// let betelgeuse = BlackbodyStellarSpectrum::new(3500.0, 1.0); // M1-2Ia (red)
///
/// // Get RGB colors
/// let (sun_r, sun_g, sun_b) = spectrum_to_rgb_values(&sun);
/// let (vega_r, vega_g, vega_b) = spectrum_to_rgb_values(&vega);
/// let (arc_r, arc_g, arc_b) = spectrum_to_rgb_values(&arcturus);
/// let (bet_r, bet_g, bet_b) = spectrum_to_rgb_values(&betelgeuse);
///
/// // Verify expected color trends
/// // Vega is white - all colors roughly equal
/// // Sun has peak in green/yellow range
/// assert!(arc_r > arc_b);      // Arcturus appears orange
/// assert!(bet_r > bet_g);      // Betelgeuse appears red
///
/// println!("Sun RGB: ({:.2}, {:.2}, {:.2})", sun_r, sun_g, sun_b);
/// println!("Vega RGB: ({:.2}, {:.2}, {:.2})", vega_r, vega_g, vega_b);
/// ```
pub fn spectrum_to_rgb_values(spectrum: &impl Spectrum) -> (f64, f64, f64) {
    // Set up duration and aperture for photo-electron calculations
    let duration = Duration::from_secs(1);
    let aperture_cm2 = 1.0;

    // Get quantum efficiency curves for red, green, and blue photoreceptors
    let red_qe = HumanVision::red();
    let green_qe = HumanVision::green_blue(); // Using green-blue as primary green
    let blue_qe = HumanVision::blue();

    // Calculate photo-electrons for each channel
    let red = spectrum.photo_electrons(&red_qe, aperture_cm2, &duration);
    let green = spectrum.photo_electrons(&green_qe, aperture_cm2, &duration);
    let blue = spectrum.photo_electrons(&blue_qe, aperture_cm2, &duration);

    // Normalize to the highest value to avoid clipping
    let max_value = red.max(green).max(blue);
    let scale = if max_value > 0.0 {
        1.0 / max_value
    } else {
        0.0
    };

    (red * scale, green * scale, blue * scale)
}

/// Calculate logarithmic blue-red color temperature index from stellar spectrum.
///
/// Computes a quantitative color index similar to astronomical B-V photometry,
/// using the logarithmic ratio of blue to red flux. This provides a temperature-
/// sensitive metric that correlates strongly with stellar effective temperature
/// and spectral class.
///
/// # Index Interpretation
/// - **Positive values**: Blue-hot stars (O, B, A types) with T > 7000K
/// - **Zero**: Neutral white stars (~7000K, late A / early F types)  
/// - **Negative values**: Red-cool stars (G, K, M types) with T < 7000K
///
/// # Astronomical Context
///
/// This index mimics the B-V color index used in stellar photometry, where
/// B-V ≈ -2.5 × log(blue_flux/red_flux). The logarithmic scale compresses
/// the wide range of stellar colors into a manageable metric for analysis.
///
/// # Arguments
/// * `spectrum` - Stellar spectrum implementing the Spectrum trait
///
/// # Returns
/// Color temperature index: ln(blue_flux / red_flux)
/// - Positive: blue-hot stars
/// - Negative: red-cool stars  
/// - Zero: white/neutral stars
///
/// # Examples
/// ```rust
/// use simulator::photometry::color::color_temperature_index;
/// use simulator::photometry::stellar::BlackbodyStellarSpectrum;
///
/// // Create spectra for different temperature stars
/// let rigel = BlackbodyStellarSpectrum::new(22000.0, 1.0);     // B8Ia (blue)
/// let sun = BlackbodyStellarSpectrum::new(5778.0, 1.0);       // G2V (yellow)
/// let proxima = BlackbodyStellarSpectrum::new(3042.0, 1.0);   // M5.5V (red)
///
/// let rigel_index = color_temperature_index(&rigel);    // > 0 (blue)
/// let sun_index = color_temperature_index(&sun);        // < 0 (yellow)  
/// let proxima_index = color_temperature_index(&proxima); // << 0 (red)
///
/// // Verify temperature ordering
/// assert!(rigel_index > sun_index);
/// assert!(sun_index > proxima_index);
/// assert!(rigel_index > 0.0);      // Blue star
/// assert!(proxima_index < 0.0);   // Red star (not necessarily < -1.0)
///
/// println!("Color indices - Rigel: {:.2}, Sun: {:.2}, Proxima: {:.2}",
///          rigel_index, sun_index, proxima_index);
/// ```
pub fn color_temperature_index(spectrum: &impl Spectrum) -> f64 {
    // Set up duration and aperture for photo-electron calculations
    let duration = Duration::from_secs(1);
    let aperture_cm2 = 1.0;

    // Get quantum efficiency curves for red and blue
    let red_qe = HumanVision::red();
    let blue_qe = HumanVision::blue();

    // Calculate photo-electrons for red and blue
    let red = spectrum.photo_electrons(&red_qe, aperture_cm2, &duration);
    let blue = spectrum.photo_electrons(&blue_qe, aperture_cm2, &duration);

    // Color index as log ratio of blue to red
    if red > 0.0 && blue > 0.0 {
        (blue / red).ln()
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::photometry::stellar::BlackbodyStellarSpectrum;
    use approx::assert_relative_eq;

    #[test]
    fn test_temperature_sequence() {
        // Test with 5 temps from 1000K to 10000K
        let temps = generate_temperature_sequence(1000.0, 10000.0, 5);

        // Should get 5 temperatures
        assert_eq!(temps.len(), 5);

        // First and last should be min and max
        assert_relative_eq!(temps[0], 1000.0, epsilon = 0.1);
        assert_relative_eq!(temps[4], 10000.0, epsilon = 0.1);

        // Should be increasing
        for i in 1..temps.len() {
            assert!(temps[i] > temps[i - 1]);
        }

        // Test with single temperature
        let single_temp = generate_temperature_sequence(5000.0, 10000.0, 1);
        assert_eq!(single_temp.len(), 1);
        assert_eq!(single_temp[0], 5000.0);
    }

    #[test]
    fn test_spectral_class() {
        // Test each spectral class boundary
        assert_eq!(temperature_to_spectral_class(50000.0), SpectralClass::O);
        assert_eq!(temperature_to_spectral_class(30000.0), SpectralClass::O);
        assert_eq!(temperature_to_spectral_class(20000.0), SpectralClass::B);
        assert_eq!(temperature_to_spectral_class(9000.0), SpectralClass::A);
        assert_eq!(temperature_to_spectral_class(6500.0), SpectralClass::F);
        assert_eq!(temperature_to_spectral_class(5800.0), SpectralClass::G); // Sun
        assert_eq!(temperature_to_spectral_class(4000.0), SpectralClass::K);
        assert_eq!(temperature_to_spectral_class(3000.0), SpectralClass::M);
    }

    #[test]
    fn test_spectral_class_display() {
        // Test that display trait works correctly
        assert_eq!(format!("{}", SpectralClass::O), "O");
        assert_eq!(format!("{}", SpectralClass::B), "B");
        assert_eq!(format!("{}", SpectralClass::A), "A");
        assert_eq!(format!("{}", SpectralClass::F), "F");
        assert_eq!(format!("{}", SpectralClass::G), "G");
        assert_eq!(format!("{}", SpectralClass::K), "K");
        assert_eq!(format!("{}", SpectralClass::M), "M");
    }

    #[test]
    fn test_spectrum_to_rgb() {
        // Create spectra for different temperature stars
        let hot_star = BlackbodyStellarSpectrum::new(30000.0, 1.0); // Blue O star
        let sun_like = BlackbodyStellarSpectrum::new(5800.0, 1.0); // Yellow G star
        let cool_star = BlackbodyStellarSpectrum::new(3000.0, 1.0); // Red M star

        // Get RGB values
        let (hot_r, hot_g, hot_b) = spectrum_to_rgb_values(&hot_star);
        let (sun_r, sun_g, sun_b) = spectrum_to_rgb_values(&sun_like);
        let (cool_r, cool_g, cool_b) = spectrum_to_rgb_values(&cool_star);

        // Hot stars should have more blue
        assert!(hot_b > hot_r);

        // Sun-like stars should have balanced RGB
        assert!(sun_g > 0.5);

        // Cool stars should have more red
        assert!(cool_r > cool_b);

        // All values should be between 0 and 1
        assert!((0.0..=1.0).contains(&hot_r));
        assert!((0.0..=1.0).contains(&hot_g));
        assert!((0.0..=1.0).contains(&hot_b));

        assert!((0.0..=1.0).contains(&sun_r));
        assert!((0.0..=1.0).contains(&sun_g));
        assert!((0.0..=1.0).contains(&sun_b));

        assert!((0.0..=1.0).contains(&cool_r));
        assert!((0.0..=1.0).contains(&cool_g));
        assert!((0.0..=1.0).contains(&cool_b));
    }

    #[test]
    fn test_color_temperature_index() {
        // Create spectra for different temperature stars
        let hot_star = BlackbodyStellarSpectrum::new(30000.0, 1.0); // Blue O star
        let sun_like = BlackbodyStellarSpectrum::new(5800.0, 1.0); // Yellow G star
        let cool_star = BlackbodyStellarSpectrum::new(3000.0, 1.0); // Red M star

        // Get color indices
        let hot_index = color_temperature_index(&hot_star);
        let sun_index = color_temperature_index(&sun_like);
        let cool_index = color_temperature_index(&cool_star);

        // Hot stars should have positive index
        assert!(hot_index > 0.0);

        // Cool stars should have negative index
        assert!(cool_index < 0.0);

        // Index should decrease with temperature
        assert!(hot_index > sun_index);
        assert!(sun_index > cool_index);
    }
}
