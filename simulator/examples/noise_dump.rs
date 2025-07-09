//! Sensor noise characteristics table generator.
//!
//! This example generates a table showing read noise and dark current levels
//! for all available sensor models at different operating temperatures.
//!
//! # Output Format
//!
//! Markdown table showing:
//! - Read noise (e⁻) at each temperature
//! - Dark current (e⁻/pixel/s) at each temperature
//! - For temperatures: -20°C, -10°C, 0°C, 10°C, 20°C
//!
//! # Usage
//!
//! ```bash
//! cargo run --example noise_dump
//! ```

use simulator::hardware::sensor::models::ALL_SENSORS;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Sensor Noise Characteristics at Different Temperatures");
    println!("====================================================");
    println!("Assumptions: Frame rate = 10 Hz (100ms exposure)");
    println!();

    // Use all available sensors
    let sensors = &*ALL_SENSORS;

    // Temperature points to evaluate
    let temperatures = vec![-20.0, -10.0, 0.0, 10.0, 20.0];

    // Frame rate for read noise estimation (10 Hz = 100ms exposure)
    let exposure_duration = std::time::Duration::from_millis(100);

    // Print header row with temperatures
    print!("| Sensor | Parameter |");
    for temp in &temperatures {
        print!(" {}°C |", temp);
    }
    println!();

    // Print separator (markdown style)
    print!("|--------|-----------|");
    for _ in &temperatures {
        print!("------|");
    }
    println!();

    // For each sensor, print read noise and dark current rows
    for config in sensors {
        // Sensor name row with read noise values
        print!("| {} | Read Noise (e⁻) |", config.name);
        for temp in &temperatures {
            let read_noise = config
                .read_noise_estimator
                .estimate(*temp, exposure_duration)
                .unwrap_or(0.0);
            print!(" {:.2} |", read_noise);
        }
        println!();

        // Dark current row
        print!("| | Dark Current (e⁻/px/s) |");
        for temp in &temperatures {
            let dark_current = config.dark_current_estimator.estimate_at_temperature(*temp);
            print!(" {:.4} |", dark_current);
        }
        println!();
    }

    Ok(())
}
