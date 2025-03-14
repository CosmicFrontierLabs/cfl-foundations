//! Image processing module for telescope simulation
//!
//! This module provides image processing utilities for the telescope simulator,
//! including convolution, filtering, thresholding, centroid calculation, and
//! other operations needed for realistic image generation and analysis.

pub mod convolve2d;
pub mod thresholding;
pub mod centroid;

// Re-export key functionality for easier access
pub use convolve2d::{convolve2d, ConvolveOptions, gaussian_kernel, ConvolveMode};
pub use centroid::{detect_stars, get_centroids, StarDetection};
pub use thresholding::{otsu_threshold, apply_threshold, connected_components, get_bounding_boxes};
