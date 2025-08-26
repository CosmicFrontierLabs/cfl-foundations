//! Zero-point magnitude calculator for telescope/sensor combinations
//!
//! This tool calculates the zero-point magnitude for different telescope and sensor
//! combinations by finding the stellar magnitude that produces exactly 1 photoelectron
//! per second. Uses binary search between magnitude 14 and 30.
//!
//! The zero-point magnitude is a fundamental calibration parameter that relates
//! instrumental measurements (counts/second) to astronomical magnitudes.

use clap::Parser;
use simulator::photometry::{photoconversion::photo_electrons, stellar::BlackbodyStellarSpectrum};
use simulator::shared_args::{SensorModel, TelescopeModel};
use simulator::{
    hardware::{
        satellite::SatelliteConfig,
        sensor::models::ALL_SENSORS,
        telescope::models::{
            COSMIC_FRONTIER_JBT_1M, COSMIC_FRONTIER_JBT_50CM, COSMIC_FRONTIER_JBT_MAX, IDEAL_100CM,
            IDEAL_50CM, OFFICINA_STELLARE_WEASEL, OPTECH_LINA_LS35, OPTECH_LINA_LS50, SMALL_50MM,
            WEASEL,
        },
        TelescopeConfig,
    },
    star_math::DEFAULT_BV,
    units::{LengthExt, Temperature, TemperatureExt, Wavelength},
};
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(
    name = "Zero-Point Calculator",
    about = "Calculates zero-point magnitude (mag at 1 e⁻/s) for telescope/sensor combinations",
    long_about = None
)]
struct Args {
    /// Specific telescope model to analyze (if not specified, analyzes all)
    #[arg(long)]
    telescope: Option<TelescopeModel>,

    /// Specific sensor model to analyze (if not specified, analyzes all)
    #[arg(long)]
    sensor: Option<SensorModel>,

    /// B-V color index for the stellar spectrum (default: 0.0, Vega-like)
    #[arg(long, default_value_t = DEFAULT_BV)]
    bv_color: f64,

    /// Convergence tolerance in magnitudes
    #[arg(long, default_value_t = 0.001)]
    tolerance: f64,

    /// Maximum iterations for binary search
    #[arg(long, default_value_t = 50)]
    max_iterations: usize,

    /// Show detailed calculation steps
    #[arg(long)]
    verbose: bool,
}

/// Calculate zero-point magnitude using binary search
///
/// Finds the magnitude at which the given satellite configuration
/// produces exactly 1 photoelectron per second.
fn find_zero_point(
    satellite: &SatelliteConfig,
    bv_color: f64,
    tolerance: f64,
    max_iterations: usize,
    verbose: bool,
) -> Result<f64, String> {
    // Binary search bounds (magnitudes)
    let mut mag_low = 5.0;
    let mut mag_high = 30.0;

    // Use telescope's clear aperture area method which accounts for obscuration
    let aperture_area = satellite.telescope.clear_aperture_area();

    // 1 second exposure
    let exposure = Duration::from_secs(1);

    let mut iteration = 0;

    while iteration < max_iterations && (mag_high - mag_low) > tolerance {
        let mag_mid = (mag_low + mag_high) / 2.0;

        // Create stellar spectrum at this magnitude
        let spectrum = BlackbodyStellarSpectrum::from_gaia_bv_magnitude(bv_color, mag_mid);

        // Calculate photoelectrons per second using combined QE from satellite
        let electrons_per_s =
            photo_electrons(&spectrum, &satellite.combined_qe, aperture_area, &exposure);

        if verbose {
            println!(
                "  Iteration {}: mag = {:.3}, e⁻/s = {:.3e}",
                iteration + 1,
                mag_mid,
                electrons_per_s
            );
        }

        // Adjust search bounds based on result
        if electrons_per_s > 1.0 {
            // Too bright, increase magnitude (dimmer)
            mag_low = mag_mid;
        } else {
            // Too dim, decrease magnitude (brighter)
            mag_high = mag_mid;
        }

        iteration += 1;
    }

    if iteration >= max_iterations {
        return Err(format!(
            "Failed to converge after {max_iterations} iterations"
        ));
    }

    // Return the midpoint as the zero-point magnitude
    Ok((mag_low + mag_high) / 2.0)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("Zero-Point Magnitude Calculator");
    println!("===============================");
    println!();
    println!("Finding magnitude at which telescope/sensor produces 1 e⁻/s");
    println!("B-V Color Index: {:.2}", args.bv_color);
    println!("Convergence Tolerance: {:.3} mag", args.tolerance);
    println!();

    // Default temperature and wavelength for satellite config
    let temperature = Temperature::from_celsius(-10.0);
    let wavelength = Wavelength::from_nanometers(550.0);

    // Determine which telescopes to analyze
    let telescopes: Vec<(&str, &TelescopeConfig)> = match args.telescope {
        Some(ref model) => match model {
            TelescopeModel::Small50mm => vec![("Small 50mm", &SMALL_50MM)],
            TelescopeModel::Demo50cm => vec![("Ideal 50cm", &IDEAL_50CM)],
            TelescopeModel::Final1m => vec![("Ideal 100cm", &IDEAL_100CM)],
            TelescopeModel::Weasel => vec![("Weasel", &WEASEL)],
            TelescopeModel::OfficinaStelareWeasel => {
                vec![("Officina Stellare Weasel", &OFFICINA_STELLARE_WEASEL)]
            }
            TelescopeModel::OptechLinaLs50 => vec![("Optech/Lina LS50", &OPTECH_LINA_LS50)],
            TelescopeModel::OptechLinaLs35 => vec![("Optech/Lina LS35", &OPTECH_LINA_LS35)],
            TelescopeModel::CosmicFrontierJbt50cm => {
                vec![("Cosmic Frontier JBT .5m", &COSMIC_FRONTIER_JBT_50CM)]
            }
            TelescopeModel::CosmicFrontierJbtMax => {
                vec![("Cosmic Frontier JBT MAX", &COSMIC_FRONTIER_JBT_MAX)]
            }
            TelescopeModel::CosmicFrontierJbt1m => {
                vec![("Cosmic Frontier JBT 1.0m", &COSMIC_FRONTIER_JBT_1M)]
            }
        },
        None => vec![
            ("Small 50mm", &SMALL_50MM),
            ("Ideal 50cm", &IDEAL_50CM),
            ("Ideal 100cm", &IDEAL_100CM),
            ("Weasel", &WEASEL),
            ("Officina Stellare Weasel", &OFFICINA_STELLARE_WEASEL),
            ("Optech/Lina LS50", &OPTECH_LINA_LS50),
            ("Optech/Lina LS35", &OPTECH_LINA_LS35),
            ("Cosmic Frontier JBT .5m", &COSMIC_FRONTIER_JBT_50CM),
            ("Cosmic Frontier JBT MAX", &COSMIC_FRONTIER_JBT_MAX),
            ("Cosmic Frontier JBT 1.0m", &COSMIC_FRONTIER_JBT_1M),
        ],
    };

    // Determine which sensors to analyze
    let sensors: Vec<&simulator::hardware::sensor::SensorConfig> = match args.sensor {
        Some(ref model) => vec![model.to_config()],
        None => ALL_SENSORS.iter().collect(),
    };

    // Print results header
    println!(
        "{:<15} {:<20} {:<15} {:<15}",
        "Telescope", "Sensor", "Zero-Point (mag)", "Aperture (m)"
    );
    println!("{:-<65}", "");

    // Calculate zero-point for each combination
    for (telescope_name, telescope) in &telescopes {
        for sensor in &sensors {
            // Create SatelliteConfig for this combination
            let satellite = SatelliteConfig::new(
                (*telescope).clone(),
                (*sensor).clone(),
                temperature,
                wavelength,
            );

            if args.verbose {
                println!("\nCalculating for {} + {}:", telescope_name, sensor.name);
            }

            match find_zero_point(
                &satellite,
                args.bv_color,
                args.tolerance,
                args.max_iterations,
                args.verbose,
            ) {
                Ok(zero_point) => {
                    println!(
                        "{:<15} {:<20} {:<15.3} {:<15.2}",
                        telescope_name,
                        sensor.name,
                        zero_point,
                        telescope.aperture.as_meters()
                    );
                }
                Err(e) => {
                    eprintln!(
                        "Error calculating for {} + {}: {}",
                        telescope_name, sensor.name, e
                    );
                }
            }
        }
    }

    println!();
    println!("Note: Zero-point is the stellar magnitude that produces 1 photoelectron/second");
    println!("      for the given telescope/sensor combination.");

    Ok(())
}
