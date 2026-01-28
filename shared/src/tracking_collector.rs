//! SSE-based tracking telemetry collection.
//!
//! Provides a simple interface for collecting tracking measurements via Server-Sent Events (SSE)
//! from an HTTP endpoint.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use crate::tracking_message::TrackingMessage;

/// Shared state between the collector and background reader thread.
struct SharedState {
    buffer: VecDeque<TrackingMessage>,
    connected: bool,
    error: Option<String>,
}

impl SharedState {
    fn new() -> Self {
        Self {
            buffer: VecDeque::with_capacity(1024),
            connected: false,
            error: None,
        }
    }
}

/// Collects tracking measurements from an SSE endpoint.
///
/// Connects to an HTTP SSE endpoint and buffers incoming TrackingMessage events.
/// The connection runs in a background thread, allowing non-blocking polling.
pub struct TrackingCollector {
    state: Arc<Mutex<SharedState>>,
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
        let state = Arc::new(Mutex::new(SharedState::new()));
        let signal: Arc<OnceLock<Result<(), String>>> = Arc::new(OnceLock::new());

        let state_clone = state.clone();
        let signal_clone = signal.clone();
        let endpoint = endpoint.to_string();

        thread::spawn(move || {
            Self::sse_reader_thread(endpoint, state_clone, signal_clone);
        });

        // Wait for initial connection result
        let start = Instant::now();
        while start.elapsed() < timeout {
            if let Some(result) = signal.get() {
                return match result {
                    Ok(()) => Ok(Self { state }),
                    Err(e) => Err(e.clone()),
                };
            }
            thread::sleep(Duration::from_millis(10));
        }

        Err(format!("Connection timed out after {timeout:?}"))
    }

    /// Connect to an SSE endpoint with default 5 second timeout.
    ///
    /// # Errors
    /// Returns an error if the connection fails or times out.
    pub fn connect(endpoint: &str) -> Result<Self, String> {
        Self::connect_with_timeout(endpoint, Duration::from_secs(5))
    }

    /// Poll for all currently available messages (non-blocking).
    ///
    /// Returns immediately with all pending messages, or an empty Vec if none.
    pub fn poll(&self) -> Vec<TrackingMessage> {
        let mut state = self.state.lock().unwrap();
        state.buffer.drain(..).collect()
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

        messages.extend(self.poll());
        messages
    }

    /// Collect exactly N messages or timeout.
    ///
    /// Blocks until `count` messages are collected or `timeout` elapses.
    /// Optionally filters messages by minimum flux.
    ///
    /// Returns the collected messages. If fewer than `count` messages were
    /// collected before timeout, returns what was collected.
    pub fn collect_n(
        &self,
        count: usize,
        timeout: Duration,
        min_flux: Option<f64>,
    ) -> Vec<TrackingMessage> {
        let start = Instant::now();
        let mut messages = Vec::with_capacity(count);

        while messages.len() < count && start.elapsed() < timeout {
            for msg in self.poll() {
                if let Some(min) = min_flux {
                    if msg.shape.flux >= min {
                        messages.push(msg);
                    }
                } else {
                    messages.push(msg);
                }
                if messages.len() >= count {
                    break;
                }
            }
            if messages.len() < count {
                thread::sleep(Duration::from_millis(10));
            }
        }

        messages
    }

    /// Wait for at least one message to arrive.
    ///
    /// Returns true if a message was received within the timeout.
    pub fn wait_for_message(&self, timeout: Duration) -> bool {
        let start = Instant::now();
        while start.elapsed() < timeout {
            if !self.poll().is_empty() {
                return true;
            }
            thread::sleep(Duration::from_millis(50));
        }
        false
    }

    /// Check if the SSE connection is currently active.
    pub fn is_connected(&self) -> bool {
        self.state.lock().unwrap().connected
    }

    /// Get the last connection error, if any.
    pub fn last_error(&self) -> Option<String> {
        self.state.lock().unwrap().error.clone()
    }

    fn sse_reader_thread(
        endpoint: String,
        state: Arc<Mutex<SharedState>>,
        signal: Arc<OnceLock<Result<(), String>>>,
    ) {
        loop {
            match ureq::get(&endpoint).call() {
                Ok(response) => {
                    {
                        let mut s = state.lock().unwrap();
                        s.connected = true;
                        s.error = None;
                    }
                    let _ = signal.set(Ok(()));

                    let reader = BufReader::new(response.into_body().into_reader());
                    for line in reader.lines() {
                        match line {
                            Ok(line) => {
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if let Ok(msg) = serde_json::from_str::<TrackingMessage>(data) {
                                        let mut s = state.lock().unwrap();
                                        s.buffer.push_back(msg);
                                        while s.buffer.len() > 10000 {
                                            s.buffer.pop_front();
                                        }
                                    }
                                }
                            }
                            Err(_) => {
                                state.lock().unwrap().connected = false;
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    let err_msg = format!("SSE connection failed: {e}");
                    {
                        let mut s = state.lock().unwrap();
                        s.connected = false;
                        s.error = Some(err_msg.clone());
                    }
                    let _ = signal.set(Err(err_msg));
                }
            }

            thread::sleep(Duration::from_secs(1));
        }
    }
}
