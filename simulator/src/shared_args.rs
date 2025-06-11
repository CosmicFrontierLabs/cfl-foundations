use crate::hardware::sensor::SensorConfig;
use crate::photometry::zodical::SolarAngularCoordinates;
use clap::{Parser, ValueEnum};
use starfield::catalogs::binary_catalog::BinaryCatalog;
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

/// Parse duration string with units (e.g., "1.5s", "150ms", "2000us", "1h", "30m")
fn parse_duration(s: &str) -> Result<Duration, String> {
    let s = s.trim();

    // Extract numeric part and unit
    let (num_str, unit) = if s.ends_with("ms") {
        (&s[..s.len() - 2], "ms")
    } else if s.ends_with("us") {
        (&s[..s.len() - 2], "us")
    } else if s.ends_with('s') {
        (&s[..s.len() - 1], "s")
    } else if s.ends_with('h') {
        (&s[..s.len() - 1], "h")
    } else if s.ends_with('m') {
        (&s[..s.len() - 1], "m")
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

    /// Enable debug output
    #[arg(long, default_value_t = false)]
    pub debug: bool,

    /// Solar elongation and coordinates for zodiacal background (format: "elongation,latitude")
    /// Defaults to the point of minimum zodiacal light brightness
    #[arg(long, default_value = DEFAULT_ZODIACAL_COORDINATES, value_parser = parse_coordinates)]
    pub coordinates: SolarAngularCoordinates,

    /// Path to binary star catalog
    #[arg(long, default_value = "gaia_mag16_multi.bin")]
    pub catalog: PathBuf,
}

/// Load a binary star catalog from the specified path
///
/// This helper function loads a BinaryCatalog and provides informative error messages
/// if loading fails. It also prints basic catalog information when debug mode is enabled.
///
/// # Arguments
/// * `catalog_path` - Path to the binary catalog file
/// * `debug` - Whether to print debug information about the loaded catalog
///
/// # Returns
/// * `Result<BinaryCatalog, Box<dyn std::error::Error>>` - The loaded catalog or error
///
/// # Example
/// ```no_run
/// use simulator::shared_args::load_catalog;
/// use std::path::PathBuf;
///
/// let catalog_path = PathBuf::from("gaia_mag16_multi.bin");
/// let catalog = load_catalog(&catalog_path, true)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn load_catalog(
    catalog_path: &PathBuf,
    debug: bool,
) -> Result<BinaryCatalog, Box<dyn std::error::Error>> {
    if debug {
        println!("Loading catalog from: {}", catalog_path.display());
    }

    let catalog = BinaryCatalog::load(catalog_path).map_err(|e| {
        format!(
            "Failed to load catalog from '{}': {}",
            catalog_path.display(),
            e
        )
    })?;

    if debug {
        println!("Loaded catalog with {} stars", catalog.len());
    }

    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::photometry::zodical::{ELONG_OF_MIN, LAT_OF_MIN};

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
}
