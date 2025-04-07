//! Algorithms for various tasks in astronomical simulations
//!
//! This module provides algorithms for point cloud alignment, feature
//! extraction, quaternion mathematics, and other computational tasks.

pub mod icp;
pub mod quaternion;

pub use icp::{iterative_closest_point, ICPResult};
pub use quaternion::Quaternion;
