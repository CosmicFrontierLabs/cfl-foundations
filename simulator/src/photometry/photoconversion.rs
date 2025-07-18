//! Photon to electron conversion utilities for detector modeling.

use crate::image_proc::airy::PixelScaledAiryDisk;
use crate::photometry::{
    spectrum::{wavelength_to_ergs, Band},
    QuantumEfficiency, Spectrum,
};
use std::time::Duration;

pub struct SpotFlux {
    pub spot: PixelScaledAiryDisk,

    // total ingegrated flux in photons or photoelectrons
    pub quantity: f64,
}

/// Calculate effective PSF and photon flux for chromatic sources.
///
/// Computes the photon-weighted average PSF size when integrating
/// over the source spectrum and detector quantum efficiency.
/// The effective PSF diameter accounts for chromatic broadening
/// due to wavelength-dependent diffraction.
///
/// # Arguments
/// * `psf` - Reference PSF with baseline FWHM and reference wavelength
/// * `spectrum` - Source spectral energy distribution
/// * `qe` - Detector quantum efficiency curve
/// * `aperture_cm2` - Telescope aperture area in cm²
/// * `duration` - Integration time
/// * `n_wavelength_samples` - Number of sub-bands for integration (default 20)
///
/// # Returns
/// Tuple of (PixelScaledAiryDisk, total_flux) where:
/// - PixelScaledAiryDisk has the effective photon-weighted diameter
/// - total_flux is the total photon count
pub fn psf_photons_photoelectrons<S: Spectrum>(
    psf: &PixelScaledAiryDisk,
    spectrum: &S,
    qe: &QuantumEfficiency,
    aperture_cm2: f64,
    duration: &Duration,
) -> (SpotFlux, SpotFlux) {
    let band = qe.band();
    // THe names in this section are a bit more punctuated than I like
    // but `p_` and `pe_` are used to indicate photon and photo-electrons
    let mut total_p_flux = 0.0;
    let mut total_pe_flux = 0.0;

    let mut pe_weighted_fwhm_sum = 0.0;
    let mut pe_weight_total = 0.0;

    let mut p_weighted_fwhm_sum = 0.0;
    let mut p_weight_total = 0.0;

    let sub_bands = band.as_n_subbands(band.width().ceil() as usize);

    for sub_band in sub_bands {
        // Compute the photons and photo-electrons flux for this sub-band
        let energy_per_photon = wavelength_to_ergs(sub_band.center());
        let irradiance = spectrum.irradiance(&sub_band);
        let p_flux = irradiance / energy_per_photon;
        let pe_flux = p_flux * qe.at(sub_band.center());

        total_p_flux += p_flux;
        total_pe_flux += pe_flux;

        let scale = if sub_band.center() < psf.reference_wavelength {
            // Below the reference wavelength, we cant assume better focusing
            1.0
        } else {
            // Airy disk radius scales linearly with wavelength
            sub_band.center() / psf.reference_wavelength
        };

        // Accrue the total relative FWHM weighted by subband PE
        pe_weighted_fwhm_sum += pe_flux * scale;
        pe_weight_total += pe_flux;

        p_weighted_fwhm_sum += p_flux * scale;
        p_weight_total += p_flux;
    }

    // Create PSF for the photos
    let p_scale = p_weighted_fwhm_sum / p_weight_total;
    let p_fwhm = psf.fwhm() * p_scale;
    let p_ref_wavelength = psf.reference_wavelength * p_scale;
    let p_psf = PixelScaledAiryDisk::with_fwhm(p_fwhm, p_ref_wavelength);

    // Create PSF for the photoelectrons
    let pe_scale = pe_weighted_fwhm_sum / pe_weight_total;
    let pe_fwhm = psf.fwhm() * pe_scale;
    let pe_ref_wavelength = psf.reference_wavelength * pe_scale;
    let pe_psf = PixelScaledAiryDisk::with_fwhm(pe_fwhm, pe_ref_wavelength);

    let flux_scale = aperture_cm2 * duration.as_secs_f64();
    let p_spot_flux = SpotFlux {
        spot: p_psf,
        quantity: total_p_flux * flux_scale,
    };
    let pe_spot_flux = SpotFlux {
        spot: pe_psf,
        quantity: total_pe_flux * flux_scale,
    };

    (p_spot_flux, pe_spot_flux)
}

/// Calculate the number of photons within a wavelength range
///
/// # Arguments
///
/// * `spectrum` - The spectrum to integrate
/// * `band` - The wavelength band to integrate over
/// * `aperture_cm2` - Collection aperture area in square centimeters
/// * `duration` - Duration of the observation
///
/// # Returns
///
/// The number of photons detected in the specified band
pub fn photons<S: Spectrum + ?Sized>(
    spectrum: &S,
    band: &Band,
    aperture_cm2: f64,
    duration: Duration,
) -> f64 {
    // Convert power to photons per second
    // E = h * c / λ, so N = P / (h * c / λ)
    // where P is power in erg/s, h is Planck's constant, c is speed of light, and λ is wavelength in cm
    let mut total_photons = 0.0;

    // Decompose the band into integer nanometer bands
    // Special case the first and last bands
    let bands = band.as_n_subbands(band.width().ceil() as usize);

    // Integrate over each wavelength in the band
    for band in bands {
        let energy_per_photon = wavelength_to_ergs(band.center());
        let irradiance = spectrum.irradiance(&band);
        total_photons += irradiance / energy_per_photon;
    }

    // Multiply by duration to get total photons detected
    total_photons * duration.as_secs_f64() * aperture_cm2
}

/// Calculate the photo-electrons obtained from this spectrum when using a sensor with a given quantum efficiency
///
/// # Arguments
/// * `spectrum` - The spectrum to integrate
/// * `qe` - The quantum efficiency of the sensor as a function of wavelength
/// * `aperture_cm2` - Collection aperture area in square centimeters
/// * `duration` - Duration of the observation
///
/// # Returns
///
/// The number of electrons detected in the specified band
pub fn photo_electrons<S: Spectrum + ?Sized>(
    spectrum: &S,
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
    let bands = {
        let band: &Band = &qe.band();
        // Calculate the number of 1nm sub-bands needed to cover the band
        // Use the Band subdivision method to create equally-sized sub-bands
        // This ensures consistent subdivision behavior and avoids code duplication
        band.as_n_subbands(band.width().ceil() as usize)
    };

    // Integrate over each wavelength in the band
    for band in bands {
        let energy_per_photon = wavelength_to_ergs(band.center());
        let photons_in_band = spectrum.irradiance(&band) / energy_per_photon;
        total_electrons += qe.at(band.center()) * photons_in_band;
    }

    // Multiply by duration to get total photons detected
    total_electrons * duration.as_secs_f64() * aperture_cm2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::photometry::{stellar::FlatStellarSpectrum, Band};
    use approx::assert_relative_eq;

    #[test]
    fn test_chromatic_monochromatic_limit() {
        // Test that chromatic PSF reduces to monochromatic when spectrum is narrow
        let achromatic_disk = PixelScaledAiryDisk::with_fwhm(1.0, 550.0);

        // Create narrow-band filter centered at 550nm
        let band = Band::from_nm_bounds(549.0, 551.0);
        let qe = QuantumEfficiency::from_notch(&band, 1.0).unwrap();

        // Create flat spectrum
        let spectrum = crate::photometry::spectrum::FlatSpectrum::unit();

        // Get effective PSF for narrow band - should be close to monochromatic
        let (photons, photoelectrons) = psf_photons_photoelectrons(
            &achromatic_disk,
            &spectrum,
            &qe,
            1.0,
            &Duration::from_secs(1),
        );

        // The effective scale should be very close to the input disk's FWHM for narrow band at reference wavelength
        assert_relative_eq!(photons.spot.fwhm(), achromatic_disk.fwhm(), epsilon = 1e-2);
        assert_relative_eq!(
            photoelectrons.spot.fwhm(),
            achromatic_disk.fwhm(),
            epsilon = 1e-2
        );
    }

    #[test]
    fn test_chromatic_broadening() {
        // Test that chromatic PSF effective scale reflects wavelength averaging
        let airy = PixelScaledAiryDisk::with_fwhm(1.0, 550.0);

        // Create broad-band filter (400-700nm)
        let band = Band::from_nm_bounds(400.0, 700.0);
        let qe = QuantumEfficiency::from_notch(&band, 1.0).unwrap();

        // Create flat spectrum
        let spectrum = crate::photometry::spectrum::FlatSpectrum::unit();

        // Get effective PSF
        let (e_spot, pe_spot) =
            psf_photons_photoelectrons(&airy, &spectrum, &qe, 1.0, &Duration::from_secs(1));

        println!(
            "Chromatic PSF FWHM: {:.3}, Photoelectron FWHM: {:.3}",
            e_spot.spot.fwhm(),
            pe_spot.spot.fwhm()
        );
        assert!(
            e_spot.spot.fwhm() > airy.fwhm(),
            "Expected chromatic psf {} to be larger than achromatic psf {}",
            e_spot.spot.fwhm(),
            airy.fwhm()
        );
        assert_relative_eq!(e_spot.spot.fwhm(), airy.fwhm(), epsilon = 0.1);
    }

    #[test]
    fn test_ir_sensitive_detector_broadening() {
        // Test that IR-sensitive detectors see wider PSF than visible-only detectors
        // due to chromatic effects - longer wavelengths have larger Airy disks
        let airy = PixelScaledAiryDisk::with_fwhm(1.0, 550.0);

        // Create a sun-like blackbody spectrum
        let spectrum = crate::photometry::stellar::BlackbodyStellarSpectrum::new(5780.0, 1e-10);

        // Create visible-only QE (400-700nm)
        let visible_wavelengths = vec![350.0, 400.0, 500.0, 600.0, 700.0, 750.0];
        let visible_efficiencies = vec![0.0, 0.5, 0.8, 0.8, 0.5, 0.0];
        let visible_qe =
            QuantumEfficiency::from_table(visible_wavelengths, visible_efficiencies).unwrap();

        // Create IR-sensitive QE (400-1000nm)
        let ir_wavelengths = vec![
            350.0, 400.0, 500.0, 600.0, 700.0, 800.0, 900.0, 1000.0, 1100.0,
        ];
        let ir_efficiencies = vec![0.0, 0.5, 0.8, 0.8, 0.7, 0.6, 0.4, 0.2, 0.0];
        let ir_qe = QuantumEfficiency::from_table(ir_wavelengths, ir_efficiencies).unwrap();

        // Get PSFs for both detectors
        let (visible_photons, visible_pe) =
            psf_photons_photoelectrons(&airy, &spectrum, &visible_qe, 1.0, &Duration::from_secs(1));

        let (ir_photons, ir_pe) =
            psf_photons_photoelectrons(&airy, &spectrum, &ir_qe, 1.0, &Duration::from_secs(1));

        // IR-sensitive detector should see wider PSF due to longer wavelengths
        assert!(
            ir_pe.spot.fwhm() > visible_pe.spot.fwhm(),
            "IR-sensitive detector PSF ({:.3}) should be wider than visible-only PSF ({:.3})",
            ir_pe.spot.fwhm(),
            visible_pe.spot.fwhm()
        );

        // The photon-weighted PSF should also be wider for IR
        assert!(
            ir_photons.spot.fwhm() > visible_photons.spot.fwhm(),
            "IR photon PSF ({:.3}) should be wider than visible photon PSF ({:.3})",
            ir_photons.spot.fwhm(),
            visible_photons.spot.fwhm()
        );

        // For IR-sensitive detector, the photoelectron PSF may be narrower than photon PSF
        // because QE is typically higher in visible wavelengths where Airy disk is smaller.
        // The key insight is that IR detectors still see wider PSF than visible-only detectors.

        println!(
            "Visible detector - Photon FWHM: {:.3}, PE FWHM: {:.3}",
            visible_photons.spot.fwhm(),
            visible_pe.spot.fwhm()
        );
        println!(
            "IR-sensitive detector - Photon FWHM: {:.3}, PE FWHM: {:.3}",
            ir_photons.spot.fwhm(),
            ir_pe.spot.fwhm()
        );
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

        let photons_count = photons(&spectrum, &band, aperture_cm2, duration);
        let electrons_count = photo_electrons(&spectrum, &qe, aperture_cm2, &duration);

        // For a perfect QE, the number of electrons should equal the number of photons
        let err = f64::abs(photons_count - electrons_count) / photons_count;

        assert!(
            err < 0.01,
            "Expected {} electrons, got {}",
            photons_count,
            electrons_count
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

        let photons_count = photons(&spectrum, &band, aperture_cm2, duration);
        let electrons_count = photo_electrons(&spectrum, &qe, aperture_cm2, &duration);

        // For 50% QE, electrons should be ~50% of photons
        let ratio = electrons_count / photons_count;

        assert_relative_eq!(ratio, 0.5, epsilon = 0.01);
    }
}
