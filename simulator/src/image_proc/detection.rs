//! Star detection algorithms for astronomical images
//!
//! This module provides a unified interface for different star detection algorithms
//! including DAO, IRAF, and naive centroiding approaches.

use ndarray::ArrayView2;
use starfield::image::starfinders::{
    DAOStarFinder, DAOStarFinderConfig, IRAFStarFinder, IRAFStarFinderConfig, StellarSource,
};

/// Enumeration of available star detection algorithms
#[derive(Debug, Clone, Copy)]
pub enum StarFinder {
    /// DAO (Daophot) photometry algorithm
    Dao,
    /// IRAF-style photometry algorithm  
    Iraf,
    /// Naive centroiding algorithm (center of mass)
    Naive,
}

/// Detect stars in an image using the specified algorithm
///
/// # Arguments
/// * `image` - The input image as a 2D array view
/// * `algorithm` - The star detection algorithm to use
/// * `cutoff` - Optional detection threshold (algorithm-dependent)
///
/// # Returns
/// A Result containing a vector of objects implementing the StellarSource trait
pub fn detect_stars(
    image: ArrayView2<u16>,
    algorithm: StarFinder,
    cutoff: Option<f64>,
) -> Result<Vec<Box<dyn StellarSource>>, String> {
    match algorithm {
        StarFinder::Dao => detect_dao(image, cutoff),
        StarFinder::Iraf => detect_iraf(image, cutoff),
        StarFinder::Naive => detect_naive(image, cutoff),
    }
}

/// Internal function for DAO star detection using DAOStarFinder
fn detect_dao(
    image: ArrayView2<u16>,
    cutoff: Option<f64>,
) -> Result<Vec<Box<dyn StellarSource>>, String> {
    // Convert u16 image to f64 for DAO algorithm
    let image_f64 = image.mapv(|x| x as f64);

    // Set default threshold if not provided
    let threshold = cutoff.unwrap_or(5.0);

    // Configure DAO star finder with reasonable defaults
    let config = DAOStarFinderConfig {
        threshold,
        fwhm: 4.0,               // Default FWHM
        ratio: 1.0,              // Circular PSF
        theta: 0.0,              // No rotation
        sigma_radius: 1.5,       // Truncation radius
        sharpness: -10.0..=10.0, // Sharpness range
        roundness: -10.0..=10.0, // Roundness range
        exclude_border: true,    // Exclude border sources
        brightest: Some(1000),   // Limit to 1000 brightest
        peakmax: None,           // No peak maximum
        min_separation: 1.0,     // Minimum separation
    };

    // Create DAO star finder and detect sources
    let star_finder = DAOStarFinder::new(config)
        .map_err(|e| format!("DAO star finder creation failed: {}", e))?;

    let stars = star_finder
        .find_stars(&image_f64, None)
        .into_iter()
        .map(|star| Box::new(star) as Box<dyn StellarSource>)
        .collect();

    Ok(stars)
}

/// Internal function for IRAF star detection using IRAFStarFinder
fn detect_iraf(
    image: ArrayView2<u16>,
    cutoff: Option<f64>,
) -> Result<Vec<Box<dyn StellarSource>>, String> {
    // Convert u16 image to f64 for IRAF algorithm
    let image_f64 = image.mapv(|x| x as f64);

    // Set default threshold if not provided
    let threshold = cutoff.unwrap_or(5.0);

    // Configure IRAF star finder with reasonable defaults
    let config = IRAFStarFinderConfig {
        threshold,
        fwhm: 4.0,             // Default FWHM
        sigma_radius: 1.5,     // Truncation radius
        minsep_fwhm: 2.5,      // Minimum separation in FWHM units
        sharpness: 0.2..=1.0,  // Sharpness range
        roundness: -1.0..=1.0, // Roundness range
        exclude_border: true,  // Exclude border sources
        brightest: Some(1000), // Limit to 1000 brightest
        peakmax: None,         // No peak maximum
        min_separation: None,  // Use default separation
    };

    // Create IRAF star finder and detect sources
    let star_finder = IRAFStarFinder::new(config)
        .map_err(|e| format!("IRAF star finder creation failed: {}", e))?;

    let stars = star_finder
        .find_stars(&image_f64, None)
        .into_iter()
        .map(|star| Box::new(star) as Box<dyn StellarSource>)
        .collect();

    Ok(stars)
}

/// Internal function for naive star detection using centroiding
fn detect_naive(
    image: ArrayView2<u16>,
    cutoff: Option<f64>,
) -> Result<Vec<Box<dyn StellarSource>>, String> {
    // Convert u16 image to f64 for centroiding algorithm
    let image_f64 = image.mapv(|x| x as f64);
    let image_view = image_f64.view();

    // Use the existing centroiding detection from the centroid module
    let detections = crate::image_proc::centroid::detect_stars(&image_view, cutoff);

    // Convert StarDetection objects to boxed StellarSource
    let stars = detections
        .into_iter()
        .map(|detection| Box::new(detection) as Box<dyn StellarSource>)
        .collect();

    Ok(stars)
}
