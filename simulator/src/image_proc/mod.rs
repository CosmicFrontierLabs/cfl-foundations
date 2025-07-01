//! Image processing module for telescope simulation
//!
//! This module provides image processing utilities for the telescope simulator,
//! including convolution, filtering, thresholding, centroid calculation, and
//! other operations needed for realistic image generation and analysis.

pub mod airy;
pub mod convolve2d;
pub mod detection;
pub mod histogram_stretch;
pub mod image;
pub mod io;
pub mod noise;
pub mod overlay;
pub mod render;
pub mod smear;

// Re-export key functionality for easier access
pub use airy::AiryDisk;
pub use convolve2d::{convolve2d, gaussian_kernel, ConvolveMode, ConvolveOptions};
pub use detection::{
    aabbs_to_tuples, apply_threshold, connected_components, detect_stars, detect_stars_unified,
    do_detections, get_bounding_boxes, get_centroids, merge_overlapping_aabbs, otsu_threshold,
    tuples_to_aabbs, union_aabbs, StarDetection, StarFinder, AABB,
};
pub use histogram_stretch::stretch_histogram;
pub use io::{save_u8_image, u16_to_u8_auto_scale, u16_to_u8_scaled};
pub use noise::{generate_noise_with_precomputed_params, generate_sensor_noise};
pub use overlay::{
    draw_bounding_boxes, draw_simple_boxes, draw_stars_with_sizes, draw_stars_with_x_markers,
    overlay_to_image,
};
