//! SSE-based tracking telemetry collection.
//!
//! Provides a simple interface for collecting tracking measurements via Server-Sent Events (SSE)
//! from an HTTP endpoint.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::tracking_message::TrackingMessage;

/// Collects tracking measurements from an SSE endpoint.
///
/// Connects to an HTTP SSE endpoint and buffers incoming TrackingMessage events.
/// The connection runs in a background thread, allowing non-blocking polling.
pub struct TrackingCollector {
    buffer: Arc<Mutex<VecDeque<TrackingMessage>>>,
    connected: Arc<Mutex<bool>>,
    error: Arc<Mutex<Option<String>>>,
}

impl TrackingCollector {
    /// Connect to an SSE endpoint and start collecting messages.
    ///
    /// Waits up to `timeout` for the connection to establish. The endpoint should
    /// be a URL like `http://host:port/tracking/events`.
    ///
    /// # Errors
    /// Returns an error if the connection fails or times out.
    pub fn connect_with_timeout(endpoint: &str, timeout: Duration) -> Result<Self, String> {
        let buffer = Arc::new(Mutex::new(VecDeque::with_capacity(1024)));
        let connected = Arc::new(Mutex::new(false));
        let error = Arc::new(Mutex::new(None));
        let connect_signal = Arc::new((Mutex::new(false), Condvar::new()));

        let buffer_clone = buffer.clone();
        let connected_clone = connected.clone();
        let error_clone = error.clone();
        let signal_clone = connect_signal.clone();
        let endpoint = endpoint.to_string();

        thread::spawn(move || {
            Self::sse_reader_thread(
                endpoint,
                buffer_clone,
                connected_clone,
                error_clone,
                signal_clone,
            );
        });

        // Wait for connection signal or timeout
        let (lock, cvar) = &*connect_signal;
        let mut signaled = lock.lock().unwrap();
        let start = Instant::now();
        while !*signaled && start.elapsed() < timeout {
            let remaining = timeout.saturating_sub(start.elapsed());
            let result = cvar.wait_timeout(signaled, remaining).unwrap();
            signaled = result.0;
        }

        // Check if connection failed
        if let Some(err) = error.lock().unwrap().as_ref() {
            return Err(err.clone());
        }

        // Check if we timed out without connecting
        if !*connected.lock().unwrap() {
            return Err(format!("Connection timed out after {timeout:?}"));
        }

        Ok(Self {
            buffer,
            connected,
            error,
        })
    }

    /// Connect to an SSE endpoint with default 5 second timeout.
    ///
    /// # Errors
    /// Returns an error if the connection fails or times out.
    pub fn connect(endpoint: &str) -> Result<Self, String> {
        Self::connect_with_timeout(endpoint, Duration::from_secs(5))
    }

    fn sse_reader_thread(
        endpoint: String,
        buffer: Arc<Mutex<VecDeque<TrackingMessage>>>,
        connected: Arc<Mutex<bool>>,
        error: Arc<Mutex<Option<String>>>,
        connect_signal: Arc<(Mutex<bool>, Condvar)>,
    ) {
        let mut caller_waiting = true;

        loop {
            match ureq::get(&endpoint).call() {
                Ok(response) => {
                    *connected.lock().unwrap() = true;
                    *error.lock().unwrap() = None;

                    // Signal that connection is established
                    if caller_waiting {
                        let (lock, cvar) = &*connect_signal;
                        *lock.lock().unwrap() = true;
                        cvar.notify_all();
                        caller_waiting = false;
                    }

                    let reader = BufReader::new(response.into_body().into_reader());

                    for line in reader.lines() {
                        match line {
                            Ok(line) => {
                                // SSE format: "data: <json>\n\n"
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if let Ok(msg) = serde_json::from_str::<TrackingMessage>(data) {
                                        let mut buf = buffer.lock().unwrap();
                                        buf.push_back(msg);
                                        // Cap buffer size to prevent memory growth
                                        while buf.len() > 10000 {
                                            buf.pop_front();
                                        }
                                    }
                                }
                                // Ignore event:, id:, retry:, and comment lines
                            }
                            Err(_) => {
                                // Connection closed or read error
                                *connected.lock().unwrap() = false;
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    *connected.lock().unwrap() = false;
                    let err_msg = format!("SSE connection failed: {e}");
                    *error.lock().unwrap() = Some(err_msg);

                    // Signal failure on first attempt
                    if caller_waiting {
                        let (lock, cvar) = &*connect_signal;
                        *lock.lock().unwrap() = true;
                        cvar.notify_all();
                        caller_waiting = false;
                    }
                }
            }

            // Reconnect after a delay
            thread::sleep(Duration::from_secs(1));
        }
    }

    /// Poll for all currently available messages (non-blocking).
    ///
    /// Returns immediately with all pending messages, or an empty Vec if none.
    pub fn poll(&self) -> Vec<TrackingMessage> {
        let mut buffer = self.buffer.lock().unwrap();
        buffer.drain(..).collect()
    }

    /// Collect messages for a specified duration.
    ///
    /// Blocks until the duration elapses, collecting all messages received
    /// during that time.
    pub fn collect(&self, duration: Duration) -> Vec<TrackingMessage> {
        let start = Instant::now();
        let mut messages = Vec::new();

        while start.elapsed() < duration {
            messages.extend(self.poll());
            thread::sleep(Duration::from_millis(10));
        }

        // Final poll to catch any last messages
        messages.extend(self.poll());
        messages
    }

    /// Check if the SSE connection is currently active.
    pub fn is_connected(&self) -> bool {
        *self.connected.lock().unwrap()
    }

    /// Get the last connection error, if any.
    pub fn last_error(&self) -> Option<String> {
        self.error.lock().unwrap().clone()
    }
}
