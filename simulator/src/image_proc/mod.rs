//! Image processing module for telescope simulation
//!
//! This module provides image processing utilities for the telescope simulator,
//! including convolution, filtering, thresholding, and other operations needed for
//! realistic image generation and analysis.

pub mod convolve2d;
pub mod thresholding;

// Re-export key functionality for easier access
pub use convolve2d::{convolve2d, ConvolveOptions};
pub use thresholding::{detect_objects, otsu_threshold, BoundingBox};