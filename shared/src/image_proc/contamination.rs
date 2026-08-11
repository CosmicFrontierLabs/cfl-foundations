//! Star contamination assessment for guide star selection
//!
//! This module provides functions to calculate the contamination between stellar sources
//! based on their PSF overlap. This is critical for determining whether two guide stars
//! will interfere with each other's centroid calculations.

use crate::image_proc::airy::PixelScaledAiryDisk;
use crate::image_proc::detection::StarDetection;

/// Calculate the separation between two stars in pixels
fn calculate_separation(source: &StarDetection, contaminant: &StarDetection) -> f64 {
    let dx = source.x - contaminant.x;
    let dy = source.y - contaminant.y;
    (dx * dx + dy * dy).sqrt()
}

/// Calculator for contamination assessment between stellar sources
#[derive(Debug, Clone)]
pub struct ContaminationCalculator {
    /// Scaled Airy disk PSF model (includes pixel scaling)
    pub psf: PixelScaledAiryDisk,
    /// Measurement aperture radius as multiple of FWHM
    pub fwhm_multiple: f64,
    /// Contamination tolerance threshold (0.0 to 1.0)
    pub tolerance: f64,
    /// Separation beyond which contamination is considered negligible (as multiple of FWHM)
    pub negligible_contamination_fwhm: f64,
}

/// Result of contamination assessment on a source star
#[derive(Debug, Clone)]
pub struct ContaminationResult {
    /// Contamination fraction at the source from the contaminant (0.0 to 1.0+)
    pub contamination_fraction: f64,
    /// Distance between source and contaminant in pixels
    pub separation: f64,
    /// Whether contamination is below threshold
    pub acceptable: bool,
}

impl ContaminationCalculator {
    /// Assess contamination on a source star from a contaminant star
    ///
    /// Calculates the contamination fraction at the source position caused by
    /// the contaminant's PSF overlap, using Gaussian PSF approximation.
    ///
    /// # Arguments
    /// * `source` - The star being contaminated
    /// * `contaminant` - The star causing contamination
    ///
    /// # Returns
    /// ContaminationResult with contamination fraction and pass/fail assessment
    pub fn assess_contamination(
        &self,
        source: &StarDetection,
        contaminant: &StarDetection,
    ) -> ContaminationResult {
        // Calculate separation
        let separation = calculate_separation(source, contaminant);

        let fwhm_pixels = self.psf.fwhm();

        // Quick check: if separation > negligible threshold, contamination is negligible
        if separation > self.negligible_contamination_fwhm * fwhm_pixels {
            return ContaminationResult {
                contamination_fraction: 0.0,
                separation,
                acceptable: true,
            };
        }

        // Always use Gaussian approximation for PSF
        let psf_value = |r_pixels: f64| -> f64 { self.psf.gaussian_approximation(r_pixels) };

        // Calculate measurement radius in pixels
        let measurement_radius = self.fwhm_multiple * fwhm_pixels;

        // Calculate contamination fraction at source from contaminant
        let contamination_fraction = if separation < measurement_radius {
            // Stars overlap significantly - use peak contamination
            contaminant.flux * psf_value(separation) / source.flux
        } else {
            // Use edge contamination estimate
            contaminant.flux * psf_value(separation - measurement_radius) / source.flux
        };

        // Check if contamination is acceptable
        let acceptable = contamination_fraction <= self.tolerance;

        ContaminationResult {
            contamination_fraction,
            separation,
            acceptable,
        }
    }

    /// Calculate integrated contamination over measurement aperture
    ///
    /// More accurate than peak contamination for close stars.
    /// Integrates the contaminating flux within the measurement aperture.
    ///
    /// # Arguments
    /// * `source` - The star being contaminated
    /// * `contaminant` - The star causing contamination
    /// * `integration_steps` - Number of integration steps in each dimension
    ///
    /// # Returns
    /// ContaminationResult with integrated contamination fraction
    pub fn integrated_contamination(
        &self,
        source: &StarDetection,
        contaminant: &StarDetection,
        integration_steps: usize,
    ) -> ContaminationResult {
        let separation = calculate_separation(source, contaminant);
        let dx = source.x - contaminant.x;
        let dy = source.y - contaminant.y;

        let fwhm_pixels = self.psf.fwhm();

        // Quick check for distant stars
        if separation > self.negligible_contamination_fwhm * fwhm_pixels {
            return ContaminationResult {
                contamination_fraction: 0.0,
                separation,
                acceptable: true,
            };
        }

        // Always use Gaussian approximation for PSF
        let psf_func = |r_pixels: f64| -> f64 { self.psf.gaussian_approximation(r_pixels) };

        let measurement_radius = self.fwhm_multiple * fwhm_pixels;

        // Integrate contamination over circular aperture around source
        // Using simple Monte Carlo integration
        let mut contamination_sum = 0.0;
        let mut source_flux_in_aperture = 0.0;

        let step_size = 2.0 * measurement_radius / integration_steps as f64;

        for i in 0..integration_steps {
            for j in 0..integration_steps {
                // Sample point relative to source
                let x_rel = -measurement_radius + (i as f64 + 0.5) * step_size;
                let y_rel = -measurement_radius + (j as f64 + 0.5) * step_size;
                let r_from_source = (x_rel * x_rel + y_rel * y_rel).sqrt();

                // Skip if outside aperture
                if r_from_source > measurement_radius {
                    continue;
                }

                // Distance from this point to contaminant
                let x_to_contaminant = x_rel + dx;
                let y_to_contaminant = y_rel + dy;
                let r_from_contaminant = (x_to_contaminant * x_to_contaminant
                    + y_to_contaminant * y_to_contaminant)
                    .sqrt();

                // Add contributions
                source_flux_in_aperture += source.flux * psf_func(r_from_source);
                contamination_sum += contaminant.flux * psf_func(r_from_contaminant);
            }
        }

        // Calculate contamination fraction
        let contamination_fraction = if source_flux_in_aperture > 0.0 {
            contamination_sum / source_flux_in_aperture
        } else {
            0.0
        };

        let acceptable = contamination_fraction <= self.tolerance;

        ContaminationResult {
            contamination_fraction,
            separation,
            acceptable,
        }
    }

    /// Simple rule-of-thumb check for contamination
    ///
    /// Uses the heuristic: separation > FWHM × (1 + sqrt(F_contaminant/F_source))
    ///
    /// # Arguments
    /// * `source` - The star being checked for contamination
    /// * `contaminant` - The star potentially causing contamination
    ///
    /// # Returns
    /// true if contamination is likely to be negligible
    pub fn quick_contamination_check(
        &self,
        source: &StarDetection,
        contaminant: &StarDetection,
    ) -> bool {
        let separation = calculate_separation(source, contaminant);

        let fwhm_pixels = self.psf.fwhm();
        let flux_ratio = contaminant.flux / source.flux;
        let required_separation = fwhm_pixels * (1.0 + flux_ratio.sqrt());

        separation > required_separation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::{LengthExt, Wavelength};
    use approx::assert_relative_eq;

    fn create_test_star(x: f64, y: f64, flux: f64) -> StarDetection {
        StarDetection {
            id: 0,
            x,
            y,
            flux,
            m_xx: 1.0,
            m_yy: 1.0,
            m_xy: 0.0,
            aspect_ratio: 1.0,
            diameter: 3.0,
        }
    }

    fn create_test_calculator() -> ContaminationCalculator {
        ContaminationCalculator {
            psf: PixelScaledAiryDisk::with_fwhm(2.5, Wavelength::from_nanometers(550.0)),
            fwhm_multiple: 2.0,
            tolerance: 0.05,
            negligible_contamination_fwhm: 5.0,
        }
    }

    #[test]
    fn test_distant_stars_no_contamination() {
        let source = create_test_star(100.0, 100.0, 1000.0);
        let contaminant = create_test_star(200.0, 200.0, 500.0);
        let calc = create_test_calculator();

        let result = calc.assess_contamination(&source, &contaminant);

        assert_eq!(result.contamination_fraction, 0.0);
        assert!(result.acceptable);
    }

    #[test]
    fn test_close_stars_contamination() {
        let source = create_test_star(100.0, 100.0, 1000.0);
        let contaminant = create_test_star(102.0, 100.0, 1000.0); // 2 pixels apart
        let calc = create_test_calculator();

        let result = calc.assess_contamination(&source, &contaminant);

        assert!(result.contamination_fraction > 0.0);
        assert!(!result.acceptable); // Should fail at default 5% threshold
    }

    #[test]
    fn test_flux_ratio_impact() {
        let faint_source = create_test_star(103.0, 100.0, 100.0);
        let bright_contaminant = create_test_star(100.0, 100.0, 10000.0);
        let calc = create_test_calculator();

        // Assess contamination on faint star from bright star
        let faint_contaminated = calc.assess_contamination(&faint_source, &bright_contaminant);

        // Assess contamination on bright star from faint star
        let bright_contaminated = calc.assess_contamination(&bright_contaminant, &faint_source);

        // Bright star should contaminate faint star much more than vice versa
        assert!(
            faint_contaminated.contamination_fraction > bright_contaminated.contamination_fraction
        );
        assert!(
            faint_contaminated.contamination_fraction / bright_contaminated.contamination_fraction
                > 10.0
        );
    }

    #[test]
    fn test_quick_check() {
        let source = create_test_star(100.0, 100.0, 1000.0);
        let contaminant_close = create_test_star(102.0, 100.0, 1000.0);
        let contaminant_far = create_test_star(110.0, 100.0, 1000.0);
        let calc = create_test_calculator();

        assert!(!calc.quick_contamination_check(&source, &contaminant_close));
        assert!(calc.quick_contamination_check(&source, &contaminant_far));
    }

    #[test]
    fn test_integrated_contamination() {
        let source1 = create_test_star(100.0, 100.0, 1000.0);
        let source2 = create_test_star(105.0, 100.0, 1000.0); // Slightly farther apart
        let calc = create_test_calculator();

        // Test contamination in both directions
        let simple_1_from_2 = calc.assess_contamination(&source1, &source2);
        let integrated_1_from_2 = calc.integrated_contamination(&source1, &source2, 30);

        let simple_2_from_1 = calc.assess_contamination(&source2, &source1);
        let integrated_2_from_1 = calc.integrated_contamination(&source2, &source1, 30);

        // Both methods should find some contamination
        assert!(simple_1_from_2.contamination_fraction > 0.0);
        assert!(integrated_1_from_2.contamination_fraction > 0.0);

        // For equal flux stars, contamination should be symmetric
        assert_relative_eq!(
            simple_1_from_2.contamination_fraction,
            simple_2_from_1.contamination_fraction,
            epsilon = 1e-10
        );
        assert_relative_eq!(
            integrated_1_from_2.contamination_fraction,
            integrated_2_from_1.contamination_fraction,
            epsilon = 1e-10
        );
    }
}
