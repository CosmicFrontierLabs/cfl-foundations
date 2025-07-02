pub mod aabb;
pub mod config;
pub mod naive;
pub mod thresholding;
pub mod unified;

pub use aabb::*;
pub use naive::{
    calculate_star_centroid, detect_stars, do_detections, get_centroids, StarDetection,
};
pub use thresholding::*;
pub use unified::{detect_stars as detect_stars_unified, StarFinder};
