//! Optimized configurations for astronomical star detection algorithms.
//!
//! This module provides pre-tuned configurations for DAOStarFinder and IRAFStarFinder
//! algorithms, specifically optimized for space telescope observations. The configurations
//! automatically adjust detection parameters based on telescope PSF characteristics and
//! noise properties for optimal stellar source detection.
//!
//! # Key Features
//!
//! - **PSF-aware tuning**: Configurations adapt to telescope Airy disk size
//! - **Noise optimization**: Thresholds scaled by background RMS levels
//! - **Space telescope focus**: Parameters optimized for diffraction-limited optics
//! - **Algorithm-specific**: Separate tuning for DAO and IRAF detection methods
//!
//! # Detection Algorithms
//!
//! ## DAOStarFinder
//! Based on Stetson's DAOPHOT photometry package. Excellent for crowded fields
//! and provides robust shape filtering (sharpness/roundness) to reject artifacts.
//!
//! ## IRAFStarFinder
//! Based on IRAF's DAOFIND task. Simpler parameter set, good for isolated stars
//! and less computationally intensive than DAOStarFinder.
//!
//! # Usage
//!
//! ```rust
//! use simulator::image_proc::detection::config::{dao_autoconfig, iraf_autoconfig};
//!
//! // Space telescope parameters
//! let airy_disk_pixels = 2.5;  // FWHM from telescope/sensor combination
//! let background_rms = 1.2;    // From background noise analysis
//! let detection_sigma = 5.0;   // 5-sigma detection threshold
//!
//! // Create optimized configurations
//! let dao_config = dao_autoconfig(airy_disk_pixels, background_rms, detection_sigma);
//! let iraf_config = iraf_autoconfig(airy_disk_pixels, background_rms, detection_sigma);
//!
//! println!("DAO threshold: {:.2}", dao_config.threshold);
//! println!("IRAF FWHM: {:.2} pixels", iraf_config.fwhm);
//! ```

use starfield::image::starfinders::{DAOStarFinderConfig, IRAFStarFinderConfig};

/// Create DAOStarFinder configuration optimized for space telescope observations.
///
/// Generates a configuration tuned for diffraction-limited space telescopes with
/// circular Airy disk PSFs. Includes conservative shape filtering to reject
/// cosmic rays, bad pixels, and image artifacts.
///
/// # Parameter Optimization
/// - **Threshold**: 1.2× higher than basic sigma threshold for reduced false positives
/// - **FWHM**: Set to 0.5× Airy disk diameter (radius-to-FWHM conversion)
/// - **Separation**: 0.8× Airy disk to allow close binary detection
/// - **Shape filters**: Moderate sharpness/roundness ranges for space telescope PSFs
///
/// # Arguments
/// * `airy_disk_pixels` - Airy disk diameter (first zero-to-zero) in pixels
/// * `background_rms` - RMS noise level of the background (e⁻ or ADU)
/// * `detection_sigma` - Detection threshold in units of sigma (typically 5.0)
///
/// # Returns
/// DAOStarFinderConfig with parameters optimized for space telescope characteristics
///
/// # Examples
/// ```rust
/// use simulator::image_proc::detection::config::dao_autoconfig;
///
/// // For 2.5 pixel FWHM space telescope
/// let config = dao_autoconfig(2.5, 1.0, 5.0);
/// assert_eq!(config.fwhm, 5.0);  // 2.0 × Airy disk diameter
/// assert_eq!(config.threshold, 6.0);  // 5σ × 1.0 RMS × 1.2 factor
/// ```
pub fn dao_autoconfig(
    airy_disk_pixels: f64,
    background_rms: f64,
    detection_sigma: f64,
) -> DAOStarFinderConfig {
    DAOStarFinderConfig {
        threshold: detection_sigma * background_rms * 1.2,
        fwhm: 2.0 * airy_disk_pixels, // Larger FWHM for better centroid accuracy
        ratio: 1.0,
        theta: 0.0,
        sigma_radius: 1.5,
        sharpness: 0.2..=5.0,
        roundness: -0.5..=0.5,
        exclude_border: false,
        brightest: None,
        peakmax: None,
        min_separation: 0.8 * airy_disk_pixels,
    }
}

/// Create IRAFStarFinder configuration optimized for space telescope observations.
///
/// Generates a configuration tuned for diffraction-limited space telescopes using
/// the simpler IRAF detection algorithm. Provides faster detection with fewer
/// parameters than DAOStarFinder.
///
/// # Parameter Optimization
/// - **Threshold**: Direct sigma threshold without additional factors
/// - **FWHM**: Set to 0.55× Airy disk diameter (slightly larger than DAO)
/// - **Separation**: 1.5× FWHM minimum separation between detections
/// - **Shape filters**: Tighter roundness constraint for space telescope PSFs
///
/// # Arguments
/// * `airy_disk_pixels` - Airy disk diameter (first zero-to-zero) in pixels
/// * `background_rms` - RMS noise level of the background (e⁻ or ADU)
/// * `detection_sigma` - Detection threshold in units of sigma (typically 5.0)
///
/// # Returns
/// IRAFStarFinderConfig with parameters optimized for space telescope characteristics
///
/// # Examples
/// ```rust
/// use simulator::image_proc::detection::config::iraf_autoconfig;
///
/// // For 2.5 pixel FWHM space telescope
/// let config = iraf_autoconfig(2.5, 1.0, 5.0);
/// assert_eq!(config.fwhm, 3.125);  // 1.25 × 2.5
/// assert_eq!(config.threshold, 5.0);  // Direct 5σ threshold
/// assert_eq!(config.minsep_fwhm, 1.5);  // 1.5× FWHM separation
/// ```
pub fn iraf_autoconfig(
    airy_disk_pixels: f64,
    background_rms: f64,
    detection_sigma: f64,
) -> IRAFStarFinderConfig {
    IRAFStarFinderConfig {
        threshold: detection_sigma * background_rms,
        fwhm: 1.25 * airy_disk_pixels, // Larger FWHM improves centroid accuracy
        sigma_radius: 1.5,
        minsep_fwhm: 1.5,      // 1.5 × FWHM separation
        sharpness: 0.2..=5.0,  // Broader range for better detection
        roundness: -0.3..=0.3, // Tight range for space telescope PSFs
        exclude_border: false,
        brightest: None,
        peakmax: None,
        min_separation: None, // Let IRAF calculate from minsep_fwhm
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_space_telescope_configs() {
        let airy_disk = 2.5; // pixels
        let background_rms = 1.2;
        let detection_sigma = 5.0;

        let dao = dao_autoconfig(airy_disk, background_rms, detection_sigma);
        assert_relative_eq!(dao.threshold, 7.2, epsilon = 1e-10); // detection_sigma * background_rms * 1.2 = 5.0 * 1.2 * 1.2
        assert_eq!(dao.fwhm, 5.0); // 2.0 * airy_disk = 2.0 * 2.5
        assert_eq!(dao.min_separation, 2.0);

        let iraf = iraf_autoconfig(airy_disk, background_rms, detection_sigma);
        assert_eq!(iraf.threshold, 6.0); // detection_sigma * background_rms = 5.0 * 1.2
        assert_eq!(iraf.fwhm, 3.125); // 1.25 * airy_disk = 1.25 * 2.5
        assert_eq!(iraf.minsep_fwhm, 1.5);
    }
}
