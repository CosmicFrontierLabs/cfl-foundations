use crate::hardware::SatelliteConfig;
use crate::image_proc::render::{
    project_stars_to_pixels, render_star_field, Renderer, RenderingResult, StarInFrame,
};
use crate::photometry::zodical::SolarAngularCoordinates;
use starfield::catalogs::StarData;
use starfield::Equatorial;
use std::time::Duration;

/// A complete scene representing a satellite's view of a star field
///
/// This struct combines the satellite configuration with a list of stars
/// projected to pixel coordinates, providing a unified interface
/// for rendering operations across different simulation tools.
///
/// # Example Usage
///
/// ```no_run
/// use simulator::{Scene, hardware::SatelliteConfig, hardware::sensor::SensorConfig,
///                 hardware::telescope::TelescopeConfig, hardware::dark_current::DarkCurrentEstimator,
///                 photometry::zodical::SolarAngularCoordinates, photometry::quantum_efficiency::QuantumEfficiency,
///                 photometry::Band};
/// use starfield::{Equatorial, catalogs::StarData};
///
/// // Create telescope configuration (50cm demo telescope)
/// let telescope = TelescopeConfig::new(
///     "Demo 50cm",
///     0.5,      // 50cm aperture
///     10.0,     // 10m focal length  
///     0.815,    // light efficiency
/// );
///
/// // Create sensor configuration (simple example)
/// let band = Band::from_nm_bounds(400.0, 700.0);
/// let qe = QuantumEfficiency::from_notch(&band, 0.8).unwrap(); // 80% QE
/// let dark_current = DarkCurrentEstimator::new(0.1, -40.0); // Low dark current
/// let sensor = SensorConfig::new(
///     "Example Sensor",
///     qe,
///     2048,     // width pixels
///     2048,     // height pixels  
///     5.0,      // 5μm pixel size
///     2.0,      // 2e- read noise
///     dark_current,
///     16,       // 16-bit depth
///     1.0,      // 1 DN per electron
///     50000.0,  // 50ke- well depth
///     10.0,     // 10 fps max
/// );
///
/// // Create satellite configuration
/// let satellite_config = SatelliteConfig::new(telescope, sensor, -10.0, 550.0);
///
/// // Create some example stars (in real usage, load from catalog)
/// let catalog_stars = vec![
///     StarData::new(1, 180.0, -30.0, 5.0, Some(0.5)), // Magnitude 5 star with B-V color
/// ];
///
/// // Define observation parameters
/// let pointing = Equatorial::from_degrees(180.0, -30.0); // RA/Dec
/// let exposure_time_s = 1.0;
///
/// // Create scene with automatic star projection
/// let scene = Scene::from_catalog(
///     satellite_config,
///     catalog_stars,
///     pointing,
///     exposure_time_s,
/// );
///
/// // Render the scene
/// let zodiacal_coords = SolarAngularCoordinates::new(90.0, 0.0).unwrap();
/// let result = scene.render(&zodiacal_coords);
///
/// println!("Rendered {} stars to {}x{} image",
///          scene.stars.len(),
///          result.quantized_image.shape()[1],
///          result.quantized_image.shape()[0]);
/// ```
#[derive(Debug, Clone)]
pub struct Scene {
    /// Satellite hardware configuration (telescope + sensor + environment)
    pub satellite_config: SatelliteConfig,

    /// Stars projected to pixel coordinates for this scene's field of view
    pub stars: Vec<StarInFrame>,

    /// Pointing center of the observation in celestial coordinates
    pub pointing_center: Equatorial,

    /// Exposure time in seconds for this observation
    pub exposure_time_s: f64,
}

impl Scene {
    /// Create a scene from a star catalog, projecting stars to pixel coordinates
    ///
    /// This method uses the shared star projection logic from render module:
    /// - Calculates PSF padding for proper edge handling
    /// - Projects stars from celestial to pixel coordinates  
    /// - Calculates expected electron flux for each star
    /// - Filters stars that fall outside the sensor bounds
    pub fn from_catalog(
        satellite_config: SatelliteConfig,
        catalog_stars: Vec<StarData>,
        pointing_center: Equatorial,
        exposure_time_s: f64,
    ) -> Self {
        // Calculate PSF padding for edge handling (same as render_star_field)
        let airy_pix = satellite_config.airy_disk_pixel_space();
        let padding = airy_pix.first_zero() * 2.0;

        // Convert stars to references (required by project_stars_to_pixels)
        let star_refs: Vec<&StarData> = catalog_stars.iter().collect();
        let exposure_duration = Duration::from_secs_f64(exposure_time_s);

        // Use shared projection function
        let projected_stars = project_stars_to_pixels(
            &star_refs,
            &pointing_center,
            &satellite_config,
            &exposure_duration,
            padding,
        );

        Self {
            satellite_config,
            stars: projected_stars,
            pointing_center,
            exposure_time_s,
        }
    }

    /// Render this scene to produce a complete rendering result
    ///
    /// Since stars are already projected to pixel coordinates, this method
    /// focuses on the image generation pipeline:
    /// - Applies PSF and renders stars to image
    /// - Adds sensor noise models  
    /// - Returns quantized image with metadata
    ///
    /// # Arguments
    /// * `zodiacal_coords` - Solar angular coordinates for zodiacal light calculation
    pub fn render(&self, zodiacal_coords: &SolarAngularCoordinates) -> RenderingResult {
        // Extract StarData references for render_star_field compatibility
        let star_data_refs: Vec<&StarData> = self.stars.iter().map(|s| &s.star).collect();
        let exposure_duration = Duration::from_secs_f64(self.exposure_time_s);

        render_star_field(
            &star_data_refs,
            &self.pointing_center,
            &self.satellite_config,
            &exposure_duration,
            zodiacal_coords,
        )
    }

    /// Create a renderer for this scene that can efficiently generate multiple exposures
    ///
    /// This method creates a Renderer that pre-computes the base star image for 1 second,
    /// allowing for efficient generation of images at different exposure times with
    /// fresh noise realizations.
    ///
    /// # Returns
    /// * `Renderer` - Configured renderer for this scene
    pub fn create_renderer(&self) -> Renderer {
        let star_data_refs: Vec<&StarData> = self.stars.iter().map(|s| &s.star).collect();

        Renderer::from_catalog(
            &star_data_refs,
            &self.pointing_center,
            self.satellite_config.clone(),
        )
    }
}
