//! Calibration pattern commands.

use serde::{Deserialize, Serialize};

/// Commands for remote control of calibration displays.
///
/// Used to command what pattern the OLED display should show
/// via REST API or ZMQ.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum PatternCommand {
    /// Display a single Gaussian spot at the specified position.
    Spot {
        /// X position in display pixels (0 = left edge)
        x: f64,
        /// Y position in display pixels (0 = top edge)
        y: f64,
        /// Full-width at half-maximum in pixels
        fwhm: f64,
        /// Peak intensity (0.0 to 1.0, where 1.0 = white)
        intensity: f64,
    },

    /// Display multiple spots simultaneously.
    SpotGrid {
        /// List of (x, y) positions in display pixels
        positions: Vec<(f64, f64)>,
        /// Full-width at half-maximum in pixels (same for all spots)
        fwhm: f64,
        /// Peak intensity (0.0 to 1.0)
        intensity: f64,
    },

    /// Display uniform gray level across entire screen.
    Uniform {
        /// Gray level (0 = black, 255 = white)
        level: u8,
    },

    /// Clear display to black.
    Clear,
}

impl Default for PatternCommand {
    fn default() -> Self {
        Self::Clear
    }
}

impl PatternCommand {
    /// Create a spot command at the center of the display.
    pub fn centered_spot(width: u32, height: u32, fwhm: f64, intensity: f64) -> Self {
        Self::Spot {
            x: width as f64 / 2.0,
            y: height as f64 / 2.0,
            fwhm,
            intensity,
        }
    }

    /// Create a grid of spots centered on the display.
    pub fn centered_grid(
        width: u32,
        height: u32,
        grid_size: usize,
        spacing: f64,
        fwhm: f64,
        intensity: f64,
    ) -> Self {
        let positions = generate_centered_grid(grid_size, spacing, width, height);
        Self::SpotGrid {
            positions,
            fwhm,
            intensity,
        }
    }
}

/// Generate a centered grid of spot positions.
///
/// Creates a grid of `grid_size × grid_size` positions centered on the display,
/// with each position separated by `grid_spacing` pixels.
pub fn generate_centered_grid(
    grid_size: usize,
    grid_spacing: f64,
    display_width: u32,
    display_height: u32,
) -> Vec<(f64, f64)> {
    let center_x = display_width as f64 / 2.0;
    let center_y = display_height as f64 / 2.0;
    let half_extent = (grid_size - 1) as f64 / 2.0;

    let mut positions = Vec::with_capacity(grid_size * grid_size);
    for row in 0..grid_size {
        for col in 0..grid_size {
            let offset_x = (col as f64 - half_extent) * grid_spacing;
            let offset_y = (row as f64 - half_extent) * grid_spacing;
            positions.push((center_x + offset_x, center_y + offset_y));
        }
    }
    positions
}
