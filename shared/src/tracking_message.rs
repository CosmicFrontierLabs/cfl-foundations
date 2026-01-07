//! Tracking message types for inter-process communication.
//!
//! Used for communicating tracked target positions between processes
//! (e.g., cam_track publishing to calibration subscribers).

use serde::{Deserialize, Serialize};

use crate::camera_interface::Timestamp;
use crate::image_proc::centroid::SpotShape;

/// A tracking update message containing the position of a tracked target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingMessage {
    /// Unique identifier for this track
    pub track_id: u32,
    /// X position in sensor coordinates (pixels)
    pub x: f64,
    /// Y position in sensor coordinates (pixels)
    pub y: f64,
    /// Timestamp when this measurement was taken
    pub timestamp: Timestamp,
    /// Spot shape characterization (flux, moments, diameter).
    /// Used for defocus mapping, PSF characterization, and radiometric calibration.
    pub shape: SpotShape,
}

impl TrackingMessage {
    /// Create a new tracking message with position and shape data.
    pub fn new(track_id: u32, x: f64, y: f64, timestamp: Timestamp, shape: SpotShape) -> Self {
        Self {
            track_id,
            x,
            y,
            timestamp,
            shape,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn make_test_shape() -> SpotShape {
        SpotShape {
            flux: 42000.0,
            m_xx: 2.5,
            m_yy: 3.0,
            m_xy: 0.1,
            aspect_ratio: 1.2,
            diameter: 5.0,
        }
    }

    #[test]
    fn test_tracking_message_serialization() {
        let shape = make_test_shape();
        let msg = TrackingMessage::new(1, 100.5, 200.5, Timestamp::new(12345, 6789), shape);

        assert_eq!(msg.track_id, 1);
        assert_relative_eq!(msg.x, 100.5);
        assert_relative_eq!(msg.y, 200.5);
        assert_eq!(msg.timestamp.seconds, 12345);
        assert_relative_eq!(msg.shape.flux, 42000.0);
        assert_relative_eq!(msg.shape.diameter, 5.0);

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("shape"));
        assert!(json.contains("flux"));
        assert!(json.contains("diameter"));

        let parsed: TrackingMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.track_id, 1);
        assert_relative_eq!(parsed.shape.flux, 42000.0);
        assert_relative_eq!(parsed.shape.diameter, 5.0);
    }
}
