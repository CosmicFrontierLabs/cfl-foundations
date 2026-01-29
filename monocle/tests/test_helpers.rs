use shared_wasm::Timestamp;
use std::time::Duration;

/// Helper function to create a test timestamp
pub fn test_timestamp() -> Timestamp {
    Timestamp::from_duration(Duration::from_millis(100))
}
