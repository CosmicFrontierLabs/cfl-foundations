//! Background keepalive for the calibration display server.
//!
//! Periodically pings the `/keepalive` endpoint on `calibrate_serve` to
//! prevent the OLED watchdog from blanking the display during long-running
//! calibration sequences.

use shared_wasm::CalibrateServerClient;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

/// Periodically pokes the calibration server keepalive endpoint.
///
/// Spawns a background tokio task on creation. The task is cancelled on drop.
/// If any keepalive calls failed during the lifetime of this struct, a warning
/// is logged when dropped.
pub struct DisplayKeepalive {
    handle: JoinHandle<()>,
    error_count: Arc<AtomicUsize>,
}

impl DisplayKeepalive {
    /// Start a background keepalive task.
    ///
    /// Sends a POST to `/keepalive` every `interval` to reset the OLED
    /// watchdog timer on the calibration server.
    pub fn spawn(client: CalibrateServerClient, interval: Duration) -> Self {
        let error_count = Arc::new(AtomicUsize::new(0));
        let error_count_clone = error_count.clone();

        let handle = tokio::spawn(async move {
            loop {
                tokio::time::sleep(interval).await;

                if let Err(e) = client.keepalive().await {
                    let count = error_count_clone.fetch_add(1, Ordering::Relaxed) + 1;
                    tracing::warn!("Display keepalive failed ({count} total): {e}");
                }
            }
        });

        Self {
            handle,
            error_count,
        }
    }
}

impl Drop for DisplayKeepalive {
    fn drop(&mut self) {
        self.handle.abort();

        let errors = self.error_count.load(Ordering::Relaxed);
        if errors > 0 {
            tracing::warn!("DisplayKeepalive: {errors} keepalive call(s) failed during lifetime");
        }
    }
}
