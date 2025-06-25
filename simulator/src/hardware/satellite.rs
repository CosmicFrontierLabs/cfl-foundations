use super::{sensor::SensorConfig, telescope::TelescopeConfig};
use crate::image_proc::airy::ScaledAiryDisk;

#[derive(Debug, Clone)]
pub struct SatelliteConfig {
    /// Telescope configuration for optical system
    pub telescope: TelescopeConfig,
    /// Sensor configuration for image capture
    pub sensor: SensorConfig,
    /// Operating temperature in degrees Celsius
    pub temperature_c: f64,
    /// Name/identifier of the satellite configuration
    pub name: String,
}

impl SatelliteConfig {
    /// Create a new satellite configuration
    pub fn new(
        name: String,
        telescope: TelescopeConfig,
        sensor: SensorConfig,
        temperature_c: f64,
    ) -> Self {
        Self {
            telescope,
            sensor,
            temperature_c,
            name,
        }
    }

    /// Get the effective collecting area accounting for telescope efficiency
    pub fn effective_collecting_area_m2(&self) -> f64 {
        self.telescope.effective_collecting_area_m2()
    }

    /// Get the plate scale in arcseconds per millimeter
    pub fn plate_scale_arcsec_per_mm(&self) -> f64 {
        self.telescope.plate_scale_arcsec_per_mm()
    }

    /// Get the plate scale in arcseconds per pixel
    pub fn plate_scale_arcsec_per_pixel(&self) -> f64 {
        let pixel_size_mm = self.sensor.pixel_size_um / 1000.0;
        self.plate_scale_arcsec_per_mm() * pixel_size_mm
    }

    /// Get the field of view in arcminutes for the sensor
    pub fn field_of_view_arcmin(&self) -> (f64, f64) {
        let arcsec_per_pixel = self.plate_scale_arcsec_per_pixel();
        let width_arcmin = (self.sensor.width_px as f64 * arcsec_per_pixel) / 60.0;
        let height_arcmin = (self.sensor.height_px as f64 * arcsec_per_pixel) / 60.0;
        (width_arcmin, height_arcmin)
    }

    /// Create a ScaledAiryDisk in pixel space for this satellite configuration
    ///
    /// # Arguments
    /// * `wavelength_nm` - Observing wavelength in nanometers
    ///
    /// # Returns
    /// A ScaledAiryDisk scaled to pixels for this telescope/sensor combination
    pub fn airy_disk_pixel_space(&self, wavelength_nm: f64) -> ScaledAiryDisk {
        // Get Airy disk radius in microns from telescope
        let airy_radius_um = self.telescope.airy_disk_radius_um(wavelength_nm);

        // Convert to pixels using sensor pixel size
        let airy_radius_pixels = airy_radius_um / self.sensor.pixel_size_um;

        // Create scaled Airy disk with pixel radius
        ScaledAiryDisk::with_first_zero(airy_radius_pixels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_satellite_config_creation() {
        let telescope = TelescopeConfig::new("Test Scope", 0.5, 2.5, 0.8);
        let sensor = crate::hardware::sensor::models::GSENSE4040BSI.clone();
        let temp = -10.0;

        let satellite = SatelliteConfig::new("Test Satellite".to_string(), telescope, sensor, temp);

        assert_eq!(satellite.name, "Test Satellite");
        assert_eq!(satellite.temperature_c, -10.0);
        assert!(satellite.effective_collecting_area_m2() > 0.0);
        assert!(satellite.plate_scale_arcsec_per_pixel() > 0.0);
    }

    #[test]
    fn test_field_of_view_calculation() {
        let telescope = TelescopeConfig::new("Test Scope", 0.5, 2.5, 0.8);
        let sensor = crate::hardware::sensor::models::GSENSE6510BSI.clone();

        let satellite = SatelliteConfig::new("FOV Test".to_string(), telescope, sensor, -10.0);

        let (width_arcmin, height_arcmin) = satellite.field_of_view_arcmin();
        assert!(width_arcmin > 0.0);
        assert!(height_arcmin > 0.0);
    }

    #[test]
    fn test_airy_disk_pixel_space() {
        let telescope = TelescopeConfig::new("Test Scope", 0.5, 2.5, 0.8);
        let sensor = crate::hardware::sensor::models::HWK4123.clone();

        let satellite =
            SatelliteConfig::new("Test Satellite".to_string(), telescope, sensor, -10.0);

        let airy_disk = satellite.airy_disk_pixel_space(550.0);

        // Airy disk should have a reasonable first zero radius in pixels
        assert!(airy_disk.first_zero() > 0.0);
        assert!(airy_disk.first_zero() < 100.0); // Should be reasonable size
    }
}
