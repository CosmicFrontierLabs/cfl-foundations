//! Algorithms for various tasks in astronomical simulations
//!
//! This module provides algorithms for point cloud alignment, feature
//! extraction, and other computational tasks.

pub mod icp;

pub use icp::{iterative_closest_point, ICPResult};
