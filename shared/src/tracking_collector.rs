//! Tracking telemetry collection from ZMQ streams.
//!
//! Provides a simple interface for collecting tracking measurements via ZMQ
//! with support for both non-blocking polling and timed collection.

use std::time::{Duration, Instant};

use crate::tracking_message::TrackingMessage;
use crate::zmq::TypedZmqSubscriber;

/// Collects tracking measurements from a ZMQ subscriber.
///
/// Wraps a `TypedZmqSubscriber<TrackingMessage>` and provides convenient
/// methods for polling and timed collection of tracking data.
pub struct TrackingCollector {
    subscriber: TypedZmqSubscriber<TrackingMessage>,
}

impl TrackingCollector {
    /// Create a new tracking collector from an existing ZMQ subscriber.
    pub fn new(subscriber: TypedZmqSubscriber<TrackingMessage>) -> Self {
        Self { subscriber }
    }

    /// Connect to a ZMQ endpoint and create a collector.
    ///
    /// # Errors
    /// Returns an error if the ZMQ connection fails.
    pub fn connect(endpoint: &str) -> Result<Self, String> {
        let subscriber = TypedZmqSubscriber::connect(endpoint)
            .map_err(|e| format!("Failed to connect to {endpoint}: {e}"))?;
        Ok(Self::new(subscriber))
    }

    /// Poll for all currently available messages (non-blocking).
    ///
    /// Returns immediately with all pending messages, or an empty Vec if none.
    pub fn poll(&self) -> Vec<TrackingMessage> {
        self.subscriber.drain()
    }

    /// Collect messages for a specified duration.
    ///
    /// Blocks until the duration elapses, collecting all messages received
    /// during that time. Uses a small sleep interval to avoid busy-waiting.
    ///
    /// # Arguments
    /// * `duration` - How long to collect messages
    ///
    /// # Returns
    /// All messages received during the collection period.
    pub fn collect(&self, duration: Duration) -> Vec<TrackingMessage> {
        let start = Instant::now();
        let mut messages = Vec::new();

        while start.elapsed() < duration {
            messages.extend(self.subscriber.drain());
            std::thread::sleep(Duration::from_millis(1));
        }

        // Final drain to catch any last messages
        messages.extend(self.subscriber.drain());
        messages
    }

    /// Get access to the underlying ZMQ subscriber.
    pub fn subscriber(&self) -> &TypedZmqSubscriber<TrackingMessage> {
        &self.subscriber
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_interface::Timestamp;
    use crate::zmq::TypedZmqPublisher;

    /// Create a connected publisher/collector pair for testing.
    /// Handles ZMQ slow joiner problem with internal sleep.
    fn create_test_pair() -> (TypedZmqPublisher<TrackingMessage>, TrackingCollector) {
        let ctx = zmq::Context::new();

        let pub_socket = ctx.socket(zmq::PUB).unwrap();
        pub_socket.bind("tcp://127.0.0.1:*").unwrap();
        let endpoint = pub_socket.get_last_endpoint().unwrap().unwrap();
        let publisher = TypedZmqPublisher::<TrackingMessage>::new(pub_socket);

        let sub_socket = ctx.socket(zmq::SUB).unwrap();
        sub_socket.connect(&endpoint).unwrap();
        sub_socket.set_subscribe(b"").unwrap();
        let collector = TrackingCollector::new(TypedZmqSubscriber::new(sub_socket));

        // ZMQ slow joiner workaround
        std::thread::sleep(Duration::from_millis(100));

        (publisher, collector)
    }

    #[test]
    fn test_poll_empty() {
        let (_pub, collector) = create_test_pair();
        let messages = collector.poll();
        assert!(messages.is_empty());
    }

    #[test]
    fn test_poll_with_messages() {
        let (publisher, collector) = create_test_pair();

        for i in 0..5 {
            let msg =
                TrackingMessage::new(1, i as f64 * 10.0, i as f64 * 20.0, Timestamp::new(i, 0));
            publisher.send(&msg).unwrap();
        }
        std::thread::sleep(Duration::from_millis(50));

        let messages = collector.poll();
        assert_eq!(messages.len(), 5);
        assert_eq!(messages[0].x, 0.0);
        assert_eq!(messages[4].x, 40.0);
    }

    #[test]
    fn test_collect_duration() {
        let (publisher, collector) = create_test_pair();

        let pub_handle = std::thread::spawn(move || {
            for i in 0..10 {
                let msg = TrackingMessage::new(1, i as f64, 0.0, Timestamp::new(i, 0));
                publisher.send(&msg).unwrap();
                std::thread::sleep(Duration::from_millis(10));
            }
        });

        let messages = collector.collect(Duration::from_millis(150));
        pub_handle.join().unwrap();

        assert_eq!(messages.len(), 10);
    }
}
