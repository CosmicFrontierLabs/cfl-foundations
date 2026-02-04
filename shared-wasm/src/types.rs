//! Core utility types.

use std::fmt;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Health check response from orin_monitor /health endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthInfo {
    pub status: String,
    pub service: String,
    pub timestamp: u64,
}

/// Timestamp structure aligned with V4L2 format.
/// Represents time as seconds and nanoseconds since an epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timestamp {
    /// Seconds component
    pub seconds: u64,
    /// Nanoseconds component (0-999,999,999)
    pub nanos: u64,
}

impl Timestamp {
    /// Create a new timestamp
    pub fn new(seconds: u64, nanos: u64) -> Self {
        Self { seconds, nanos }
    }

    /// Create a timestamp from a Duration since epoch
    pub fn from_duration(duration: Duration) -> Self {
        let total_nanos = duration.as_nanos();
        let seconds = (total_nanos / 1_000_000_000) as u64;
        let nanos = (total_nanos % 1_000_000_000) as u64;
        Self { seconds, nanos }
    }

    /// Convert to Duration
    pub fn to_duration(&self) -> Duration {
        Duration::new(self.seconds, self.nanos as u32)
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{:09}", self.seconds, self.nanos)
    }
}

/// Spot shape characterization without position.
///
/// Contains flux, shape moments, and size measurements extracted from a centroid
/// calculation. Used for transmitting shape data separately from position
/// (e.g., in tracking messages where frame-relative position is stored separately).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotShape {
    /// Total flux (sum of all pixel intensities)
    pub flux: f64,
    /// Second central moment μ₂₀ (variance in x-direction)
    pub m_xx: f64,
    /// Second central moment μ₀₂ (variance in y-direction)
    pub m_yy: f64,
    /// Second central moment μ₁₁ (covariance between x and y)
    pub m_xy: f64,
    /// Aspect ratio (λ₁/λ₂) from eigenvalues of moment matrix
    pub aspect_ratio: f64,
    /// Estimated object diameter in pixels
    pub diameter: f64,
}
