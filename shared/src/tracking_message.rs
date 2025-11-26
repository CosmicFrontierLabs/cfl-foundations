//! Tracking message types for inter-process communication.
//!
//! Used for communicating tracked target positions between processes
//! (e.g., cam_track publishing to calibration subscribers).

use serde::{Deserialize, Serialize};

use crate::camera_interface::Timestamp;

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
}

impl TrackingMessage {
    /// Create a new tracking message
    pub fn new(track_id: u32, x: f64, y: f64, timestamp: Timestamp) -> Self {
        Self {
            track_id,
            x,
            y,
            timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracking_message_serialization() {
        let msg = TrackingMessage::new(1, 100.5, 200.5, Timestamp::new(12345, 6789));
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: TrackingMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.track_id, 1);
        assert!((parsed.x - 100.5).abs() < 1e-10);
        assert!((parsed.y - 200.5).abs() < 1e-10);
        assert_eq!(parsed.timestamp.seconds, 12345);
    }
}
