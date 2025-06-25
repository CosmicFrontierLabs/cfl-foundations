use crate::hardware::sensor::SensorConfig;
use crate::photometry::zodical::SolarAngularCoordinates;
use clap::{Parser, ValueEnum};
use starfield::catalogs::binary_catalog::{BinaryCatalog, MinimalStar};
use std::path::PathBuf;
use std::time::Duration;

/// Parse coordinates string in format "elongation,latitude"
fn parse_coordinates(s: &str) -> Result<SolarAngularCoordinates, String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return Err("Coordinates must be in format 'elongation,latitude'".to_string());
    }

    let elongation = parts[0]
        .trim()
        .parse::<f64>()
        .map_err(|_| "Invalid elongation value".to_string())?;
    let latitude = parts[1]
        .trim()
        .parse::<f64>()
        .map_err(|_| "Invalid latitude value".to_string())?;

    SolarAngularCoordinates::new(elongation, latitude)
        .map_err(|e| format!("Invalid coordinates: {}", e))
}

/// Default coordinates string for zodiacal light minimum
/// Uses the values from ELONG_OF_MIN (165.0) and LAT_OF_MIN (75.0)
const DEFAULT_ZODIACAL_COORDINATES: &str = "165.0,75.0";

/// Additional bright stars to inject into catalogs (embedded at compile time)
const ADDITIONAL_BRIGHT_STARS_CSV: &str = include_str!("../data/missing_bright_stars.csv");

/// Parse CSV data into MinimalStar instances
///
/// Expected CSV format: RA_deg,Dec_deg,Gaia_magnitude
/// Header line is skipped automatically
fn parse_additional_stars() -> Result<Vec<MinimalStar>, Box<dyn std::error::Error>> {
    let mut stars = Vec::new();
    let mut current_id = u64::MAX; // Start from maximum possible value and count backwards

    for (line_num, line) in ADDITIONAL_BRIGHT_STARS_CSV.lines().enumerate() {
        // Skip header line
        if line_num == 0 {
            continue;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() != 3 {
            return Err(format!(
                "Invalid CSV format at line {}: expected 3 columns, got {}",
                line_num + 1,
                parts.len()
            )
            .into());
        }

        let ra_deg = parts[0]
            .trim()
            .parse::<f64>()
            .map_err(|e| format!("Invalid RA at line {}: {}", line_num + 1, e))?;
        let dec_deg = parts[1]
            .trim()
            .parse::<f64>()
            .map_err(|e| format!("Invalid Dec at line {}: {}", line_num + 1, e))?;
        let magnitude = parts[2]
            .trim()
            .parse::<f64>()
            .map_err(|e| format!("Invalid magnitude at line {}: {}", line_num + 1, e))?;

        stars.push(MinimalStar::new(current_id, ra_deg, dec_deg, magnitude));
        current_id = current_id.saturating_sub(1); // Count backwards, protecting against underflow
    }

    Ok(stars)
}

/// Parse duration string with units (e.g., "1.5s", "150ms", "2000us", "1h", "30m")
fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();

    // Extract numeric part and unit
    let (num_str, unit) = if let Some(stripped) = s.strip_suffix("ms") {
        (stripped, "ms")
    } else if let Some(stripped) = s.strip_suffix("us") {
        (stripped, "us")
    } else if let Some(stripped) = s.strip_suffix('s') {
        (stripped, "s")
    } else if let Some(stripped) = s.strip_suffix('h') {
        (stripped, "h")
    } else if let Some(stripped) = s.strip_suffix('m') {
        (stripped, "m")
    } else {
        // Default to seconds if no unit specified
        (s, "s")
    };

    let value: f64 = num_str
        .parse()
        .map_err(|_| format!("Invalid numeric value: {}", num_str))?;

    if value < 0.0 {
        return Err("Duration cannot be negative".to_string());
    }

    let duration = match unit {
        "us" => Duration::from_micros((value * 1.0) as u64),
        "ms" => Duration::from_millis((value * 1.0) as u64),
        "s" => Duration::from_secs_f64(value),
        "m" => Duration::from_secs_f64(value * 60.0),
        "h" => Duration::from_secs_f64(value * 3600.0),
        _ => return Err(format!("Unknown time unit: {}", unit)),
    };

    Ok(duration)
}

/// Wrapper for Duration that implements Clone and has a nice Display
#[derive(Debug, Clone)]
pub struct DurationArg(pub Duration);

impl std::str::FromStr for DurationArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_duration(s).map(DurationArg)
    }
}

impl std::fmt::Display for DurationArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let duration = self.0;
        let total_ms = duration.as_millis();

        if total_ms >= 1000 && total_ms % 1000 == 0 {
            write!(f, "{}s", total_ms / 1000)
        } else if total_ms >= 1000 {
            write!(f, "{:.3}s", duration.as_secs_f64())
        } else {
            write!(f, "{}ms", total_ms)
        }
    }
}

impl Default for DurationArg {
    fn default() -> Self {
        DurationArg(Duration::from_secs(1))
    }
}

/// Available sensor models for selection
#[derive(Debug, Clone, ValueEnum)]
pub enum SensorModel {
    /// GSENSE4040BSI CMOS sensor (4096x4096, 9μm pixels)
    Gsense4040bsi,
    /// GSENSE6510BSI CMOS sensor (3200x3200, 6.5μm pixels) - Default
    Gsense6510bsi,
    /// HWK4123 CMOS sensor (4096x2300, 4.6μm pixels)
    Hwk4123,
    /// Sony IMX455 Full-frame BSI CMOS sensor (9568x6380, 3.75μm pixels)
    Imx455,
}

impl std::fmt::Display for SensorModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensorModel::Gsense4040bsi => write!(f, "gsense4040bsi"),
            SensorModel::Gsense6510bsi => write!(f, "gsense6510bsi"),
            SensorModel::Hwk4123 => write!(f, "hwk4123"),
            SensorModel::Imx455 => write!(f, "imx455"),
        }
    }
}

impl SensorModel {
    /// Get the corresponding SensorConfig for the selected model
    pub fn to_config(&self) -> &'static SensorConfig {
        match self {
            SensorModel::Gsense4040bsi => &crate::hardware::sensor::models::GSENSE4040BSI,
            SensorModel::Gsense6510bsi => &crate::hardware::sensor::models::GSENSE6510BSI,
            SensorModel::Hwk4123 => &crate::hardware::sensor::models::HWK4123,
            SensorModel::Imx455 => &crate::hardware::sensor::models::IMX455,
        }
    }
}

/// Common arguments shared across multiple simulation binaries
#[derive(Parser, Debug, Clone)]
pub struct SharedSimulationArgs {
    /// Exposure time (e.g., "1s", "500ms", "0.1s")
    #[arg(long, default_value = "1s")]
    pub exposure: DurationArg,

    /// Wavelength in nanometers
    #[arg(long, default_value_t = 550.0)]
    pub wavelength: f64,

    /// Sensor temperature in degrees Celsius for dark current calculation
    #[arg(long, default_value_t = 20.0)]
    pub temperature: f64,

    /// Solar elongation and coordinates for zodiacal background (format: "elongation,latitude")
    /// Defaults to the point of minimum zodiacal light brightness
    #[arg(long, default_value = DEFAULT_ZODIACAL_COORDINATES, value_parser = parse_coordinates)]
    pub coordinates: SolarAngularCoordinates,

    /// Path to binary star catalog
    #[arg(long, default_value = "gaia_mag16_multi.bin")]
    pub catalog: PathBuf,

    /// Noise multiple for detection cutoff (detection threshold = mean_noise * noise_multiple)
    #[arg(long, default_value_t = 5.0)]
    pub noise_multiple: f64,
}

impl SharedSimulationArgs {
    /// Load a binary star catalog from the configured path and union with additional bright stars
    ///
    /// This method loads a BinaryCatalog and adds additional bright stars from embedded CSV data.
    ///
    /// # Returns
    /// * `Result<BinaryCatalog, Box<dyn std::error::Error>>` - The loaded catalog with additional stars or error
    ///
    /// # Example
    /// ```no_run
    /// use simulator::shared_args::SharedSimulationArgs;
    /// use clap::Parser;
    ///
    /// let args = SharedSimulationArgs::parse();
    /// let catalog = args.load_catalog()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn load_catalog(&self) -> Result<BinaryCatalog, Box<dyn std::error::Error>> {
        let mut catalog = BinaryCatalog::load(&self.catalog).map_err(|e| {
            format!(
                "Failed to load catalog from '{}': {}",
                self.catalog.display(),
                e
            )
        })?;

        // Parse and add additional bright stars
        let additional_stars =
            parse_additional_stars().expect("Failed to parse embedded additional bright stars CSV");

        // Get existing stars and combine with additional ones
        let mut all_stars = catalog.stars().to_vec();
        all_stars.extend(additional_stars);

        // Create new catalog with combined stars
        let updated_description = format!(
            "{} + {} additional bright stars",
            catalog.description(),
            all_stars.len() - catalog.len()
        );
        catalog = BinaryCatalog::from_stars(all_stars, &updated_description);

        Ok(catalog)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::photometry::zodical::{ELONG_OF_MIN, LAT_OF_MIN};
    use starfield::catalogs::StarPosition;

    #[test]
    fn test_default_coordinates_match_zodiacal_constants() {
        // Parse the default coordinates string
        let parsed = parse_coordinates(DEFAULT_ZODIACAL_COORDINATES)
            .expect("Default coordinates string should be valid");

        // Ensure they match the zodiacal light minimum constants
        assert_eq!(
            parsed.elongation(),
            ELONG_OF_MIN,
            "Default elongation should match ELONG_OF_MIN"
        );
        assert_eq!(
            parsed.latitude(),
            LAT_OF_MIN,
            "Default latitude should match LAT_OF_MIN"
        );

        // Also verify the string format is what we expect
        assert_eq!(DEFAULT_ZODIACAL_COORDINATES, "165.0,75.0");
    }

    #[test]
    fn test_duration_parsing() {
        // Test various duration formats
        assert_eq!(parse_duration("1s").unwrap(), Duration::from_secs(1));
        assert_eq!(parse_duration("500ms").unwrap(), Duration::from_millis(500));
        assert_eq!(
            parse_duration("1.5s").unwrap(),
            Duration::from_secs_f64(1.5)
        );
        assert_eq!(parse_duration("2m").unwrap(), Duration::from_secs(120));
        assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));

        // Test error cases
        assert!(parse_duration("-1s").is_err());
        assert!(parse_duration("invalid").is_err());
    }

    #[test]
    fn test_parse_additional_stars() {
        // Test the actual embedded CSV parsing
        let stars = parse_additional_stars().expect("Should parse embedded CSV successfully");

        // Verify we got some stars (the actual CSV should have many)
        assert!(!stars.is_empty(), "Should have parsed at least some stars");

        // Check that IDs are assigned backwards from u64::MAX
        if stars.len() >= 2 {
            let first_star = &stars[0];
            let second_star = &stars[1];
            assert_eq!(first_star.id, u64::MAX, "First star should have max u64 ID");
            assert_eq!(
                second_star.id,
                u64::MAX - 1,
                "Second star should have max-1 ID"
            );
        }

        // Verify all stars have valid coordinates and magnitudes
        for star in &stars {
            // RA should be in range [0, 360) degrees
            assert!(
                star.ra() >= 0.0 && star.ra() < 360.0,
                "RA {} should be in range [0, 360)",
                star.ra()
            );

            // Dec should be in range [-90, 90] degrees
            assert!(
                star.dec() >= -90.0 && star.dec() <= 90.0,
                "Dec {} should be in range [-90, 90]",
                star.dec()
            );

            // Magnitude should be reasonable (very bright stars, so negative to ~6)
            assert!(
                star.magnitude >= -2.0 && star.magnitude <= 7.0,
                "Magnitude {} should be reasonable for bright stars",
                star.magnitude
            );

            // ID should be counting backwards from u64::MAX
            assert!(
                star.id >= u64::MAX - stars.len() as u64,
                "Star ID {} should be in expected range",
                star.id
            );
        }

        println!("Successfully parsed {} additional stars", stars.len());
        if !stars.is_empty() {
            let first = &stars[0];
            println!(
                "First star: ID={}, RA={:.6}°, Dec={:.6}°, Mag={:.2}",
                first.id,
                first.ra(),
                first.dec(),
                first.magnitude
            );
        }
    }
}
