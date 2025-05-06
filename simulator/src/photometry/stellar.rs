//! Stellar spectrum models for astronomical photometry
//!
//! This module provides implementations of the Spectrum trait
//! for modeling stellar spectra.

use std::time::Duration;

use super::quantum_efficiency::QuantumEfficiency;
use super::spectrum::{nm_sub_bands, wavelength_to_ergs, Band, Spectrum, CGS};

/// A flat stellar spectrum with constant spectral flux density
///
/// This represents a source with the same energy per unit frequency
/// across all wavelengths. Commonly used for simple stellar modeling.
#[derive(Debug, Clone)]
pub struct FlatStellarSpectrum {
    /// Spectral flux density in erg s⁻¹ cm⁻² Hz⁻¹
    spectral_flux_density: f64,
}

impl FlatStellarSpectrum {
    /// Create a new FlatStellarSpectrum with a constant spectral flux density
    ///
    /// # Arguments
    ///
    /// * `spectral_flux_density` - The spectral flux density in (erg s⁻¹ cm⁻² Hz⁻¹)
    ///
    /// # Returns
    ///
    /// A new FlatStellarSpectrum with the specified flux density
    pub fn new(spectral_flux_density: f64) -> Self {
        Self {
            spectral_flux_density,
        }
    }

    /// Create a new FlatStellarSpectrum from an AB magnitude
    ///
    /// # Arguments
    ///
    /// * `ab_mag` - The AB magnitude of the source
    ///
    /// # Returns
    ///
    /// A new FlatStellarSpectrum with the spectral flux density corresponding to the given AB magnitude
    pub fn from_ab_mag(ab_mag: f64) -> Self {
        // Convert AB magnitude to flux density
        // F_ν = F_ν,0 * 10^(-0.4 * AB)
        // where F_ν,0 is the zero point (3631 Jy for AB mag system)
        let spectral_flux_density = CGS::AB_ZERO_POINT_FLUX_DENSITY * 10f64.powf(-0.4 * ab_mag);
        Self::new(spectral_flux_density)
    }

    /// Create a new FlatStellarSpectrum from a GaiaV2/V3 value
    /// # Arguments
    ///
    /// `gaia_magnitude` - The GaiaV2/V3 magnitude of the source
    /// # Returns
    ///
    /// A new FlatStellarSpectrum with the spectral flux density corresponding to the given Gaia magnitude
    pub fn from_gaia_magnitude(gaia_magnitude: f64) -> Self {
        // Convert Gaia magnitude to flux density
        // Same scaling, but slightly different zero-point definition
        Self::from_ab_mag(gaia_magnitude + 0.12)
    }
}

impl Spectrum for FlatStellarSpectrum {
    fn spectral_irradiance(&self, wavelength_nm: f64) -> f64 {
        // For a flat spectrum in frequency space, the spectral flux density
        // constant spectral irradiance

        // Ensure wavelength is positive
        if wavelength_nm <= 0.0 {
            return 0.0;
        }

        // erg s⁻¹ cm⁻² Hz⁻¹
        self.spectral_flux_density
    }

    fn irradiance(&self, band: &Band) -> f64 {
        // Integrate the spectral irradiance over the wavelength range
        // and multiply by the aperture area

        if band.lower_nm >= band.upper_nm || band.lower_nm <= 0.0 {
            return 0.0;
        }

        // Convert band to frequency bounds
        let (lower_freq, upper_freq) = band.frequency_bounds();

        self.spectral_flux_density * (upper_freq - lower_freq)
    }

    fn photons(&self, band: &Band, aperture_cm2: f64, duration: std::time::Duration) -> f64 {
        // Convert power to photons per second
        // E = h * c / λ, so N = P / (h * c / λ)
        // where P is power in erg/s, h is Planck's constant, c is speed of light, and λ is wavelength in cm
        let mut total_photons = 0.0;

        // Decompose the band into integer nanometer bands
        // Special case the first and last bands
        let bands = nm_sub_bands(band);

        // Integrate over each wavelength in the band
        for band in bands {
            let energy_per_photon = wavelength_to_ergs(band.center());
            let irradiance = self.irradiance(&band);
            total_photons += irradiance / energy_per_photon;
        }

        // Multiply by duration to get total photons detected
        total_photons * duration.as_secs_f64() * aperture_cm2
    }

    fn photo_electrons(
        &self,
        qe: &QuantumEfficiency,
        aperture_cm2: f64,
        duration: &Duration,
    ) -> f64 {
        // Convert power to photons per second
        // E = h * c / λ, so N = P / (h * c / λ)
        // where P is power in erg/s, h is Planck's constant, c is speed of light, and λ is wavelength in cm
        let mut total_electrons = 0.0;

        // Decompose the band into integer nanometer bands
        // Special case the first and last bands
        let bands = nm_sub_bands(&qe.band());

        // Integrate over each wavelength in the band
        for band in bands {
            let energy_per_photon = wavelength_to_ergs(band.center());
            let photons_in_band = self.irradiance(&band) / energy_per_photon;
            total_electrons += qe.at(band.center()) * photons_in_band;
        }

        // Multiply by duration to get total photons detected
        total_electrons * duration.as_secs_f64() * aperture_cm2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_aaron_matching_photoelec() {
        let mag_to_photons = vec![
            (0.0, 3_074_446.0),
            (10.0, 0.0001 * 3_074_446.0),
            (12.0, 48.0),
        ];
        let band: Band = Band::from_nm_bounds(400.0, 700.0);
        let qe = QuantumEfficiency::from_notch(&band, 1.0).unwrap();

        for (mag, expected_electrons) in mag_to_photons.iter() {
            // Calculate number of photons in 400-700nm if a 12th mag star
            let spectrum = FlatStellarSpectrum::from_ab_mag(*mag);

            // Assume 1 cm² aperture and 1 second duration
            let aperture_cm2 = 1.0;
            let duration = std::time::Duration::from_secs_f64(1.0);
            let electrons = spectrum.photo_electrons(&qe, aperture_cm2, &duration);

            let error = f64::abs(electrons - *expected_electrons) / *expected_electrons;

            assert!(
                error < 0.02,
                "For mag {}: Got {} Expected ~{}",
                mag,
                electrons,
                expected_electrons
            );
        }
    }

    #[test]
    fn test_aaron_matching() {
        let mag_to_photons = vec![
            (0.0, 3_074_446.0),
            (10.0, 0.0001 * 3_074_446.0),
            (12.0, 48.0),
        ];

        for (mag, expected_photons) in mag_to_photons.iter() {
            // Calculate number of photons in 400-700nm if a 12th mag star
            let spectrum = FlatStellarSpectrum::from_ab_mag(*mag);

            let band = Band::from_nm_bounds(400.0, 700.0);

            // Assume 1 cm² aperture and 1 second duration
            let aperture_cm2 = 1.0;
            let duration = std::time::Duration::from_secs(1);
            let photons = spectrum.photons(&band, aperture_cm2, duration);

            let error = f64::abs(photons - *expected_photons) / *expected_photons;

            assert!(
                error < 0.02,
                "For mag {}: Got {} Expected ~{}",
                mag,
                photons,
                expected_photons
            );
        }
    }

    #[test]
    fn test_flat_stellar_spectrum() {
        // Test creating from Jansky value
        let spectrum = FlatStellarSpectrum::new(3631.0);

        // Test irradiance at different wavelengths
        // The spectral irradiance in wavelength units varies with wavelength
        // even though the frequency spectrum is flat
        let spec_irr_500 = spectrum.spectral_irradiance(500.0);
        let spec_irr_1000 = spectrum.spectral_irradiance(1000.0);

        // Irradiance should be higher at shorter wavelengths (F_λ ∝ 1/λ²)
        assert_eq!(spec_irr_500, spec_irr_1000);

        // Test creating from AB magnitude
        let spectrum_ab = FlatStellarSpectrum::from_ab_mag(0.0);

        // AB mag of 0 should give the same result as 3631 Jy
        assert_relative_eq!(
            spectrum_ab.spectral_irradiance(500.0),
            CGS::JANSKY_IN_CGS * 3631.0,
            epsilon = 1e-5
        );
    }

    #[test]
    fn test_printout_photons() {
        let wavelengths = vec![400.0, 500.0, 600.0, 700.0];
        let spectrum = FlatStellarSpectrum::from_ab_mag(0.0);
        let aperture_cm2 = 1.0; // 1 cm² aperture
        let duration = std::time::Duration::from_secs(1); // 1 second observation

        for wavelength in wavelengths {
            // Make a band that is at the wavelength +- 1THz
            let band = Band::centered_on(wavelength, 1e12);
            let irradiance = spectrum.irradiance(&band);
            let photons = spectrum.photons(&band, aperture_cm2, duration);
            println!(
                "Wavelength: {} nm, Irradiance: {} Photons: {:.2}",
                wavelength, irradiance, photons
            );
        }
    }

    #[test]
    fn test_stellar_photon_spectrum() {
        let spectrum = FlatStellarSpectrum::from_gaia_magnitude(10.0);

        let band1 = Band::centered_on(400.0, 1e12);
        let band2 = Band::centered_on(800.0, 1e12);

        // Calculate photons in each band
        let photons1 = spectrum.photons(&band1, 1.0, std::time::Duration::from_secs(1));
        let photons2 = spectrum.photons(&band2, 1.0, std::time::Duration::from_secs(1));

        // Should be the same number 2x frequency == 1/2 wavelength == 2x photons
        println!(
            "Photons in band1 ({}nm): {}, band2 ({}nm): {}",
            band1.lower_nm, photons1, band2.lower_nm, photons2
        );
        assert_relative_eq!(photons1 * 2.0, photons2, epsilon = 1e-5);
    }

    #[test]
    fn test_stellar_spectrum_scaling() {
        let spectrum = FlatStellarSpectrum::from_ab_mag(0.0);
        // Test dimmer star (AB mag = 5.0)
        let spectrum_dim = FlatStellarSpectrum::from_ab_mag(5.0);

        // Should be 100x dimmer (5 mags = factor of 100)
        assert_relative_eq!(
            spectrum.spectral_irradiance(500.0) / spectrum_dim.spectral_irradiance(500.0),
            100.0,
            epsilon = 1e-5
        );
    }

    #[test]
    fn test_flat_stellar_irradiance() {
        // Create a flat spectrum with a known flux density
        let spectrum = FlatStellarSpectrum::from_ab_mag(0.0);

        // Test whole range
        let band = Band::from_nm_bounds(400.0, 700.0);
        let power1 = spectrum.irradiance(&band);

        // Test partial range
        let band2 = Band::from_nm_bounds(450.0, 600.0);
        let power2 = spectrum.irradiance(&band2);

        // Ensure non-zero values
        assert!(power1 > 0.0);
        assert!(power2 > 0.0);

        // First band should have more power (wider wavelength range)
        assert!(power1 > power2);

        // Test range outside spectrum
        let band3 = Band::from_nm_bounds(0.1, 0.2); // Very small wavelengths but not negative
        assert!(spectrum.irradiance(&band3) > 0.0);
    }

    #[test]
    fn test_photoelectron_math_100percent() {
        let aperture_cm2 = 1.0; // 1 cm² aperture
        let duration = std::time::Duration::from_secs(1); // 1 second observation

        let band = Band::from_nm_bounds(400.0, 600.0);
        // Make a pretend QE that is perfect in the 400-600nm range
        let qe = QuantumEfficiency::from_notch(&band, 1.0).unwrap();

        // Create a flat spectrum with a known flux density
        let spectrum = FlatStellarSpectrum::from_ab_mag(0.0);

        let photons = spectrum.photons(&band, aperture_cm2, duration);
        let electrons = spectrum.photo_electrons(&qe, aperture_cm2, &duration);

        // For a perfect QE, the number of electrons should equal the number of photons
        let err = f64::abs(photons - electrons) / photons;

        assert!(
            err < 0.01,
            "Expected {} electrons, got {}",
            photons,
            electrons
        );
    }

    #[test]
    fn test_photoelectron_math_50_percent() {
        let aperture_cm2 = 1.0; // 1 cm² aperture
        let duration = std::time::Duration::from_secs(1); // 1 second observation

        let band = Band::from_nm_bounds(400.0, 600.0);
        // Make a pretend QE with 50% efficiency in the 400-600nm range
        let qe = QuantumEfficiency::from_notch(&band, 0.5).unwrap();

        // Create a flat spectrum with a known flux density
        let spectrum = FlatStellarSpectrum::from_ab_mag(0.0);

        let photons = spectrum.photons(&band, aperture_cm2, duration);
        let electrons = spectrum.photo_electrons(&qe, aperture_cm2, &duration);

        // For 50% QE, electrons should be ~50% of photons
        let ratio = electrons / photons;

        assert_relative_eq!(ratio, 0.5, epsilon = 0.01);
    }
}
