//! PI GCS (General Command Set) Protocol Implementation
//!
//! This module implements the low-level GCS 2.0 protocol used by Physik Instrumente (PI)
//! controllers including the E-727 digital piezo controller.
//!
//! # Protocol Overview
//!
//! GCS is a text-based protocol where commands are 3 characters (e.g., `MOV`, `POS`)
//! and queries append a `?` (e.g., `POS?`). Messages are terminated with LF (0x0A).
//!
//! ## Response Format
//!
//! Single-value responses: `<AxisID>=<Value>\n`
//!
//! Multi-line responses use ` \n` (space+LF) as line separators, with the final
//! line ending in just `\n`:
//!
//! ```text
//! 1=100.5 \n
//! 2=200.3 \n
//! 3=150.0\n
//! ```
//!
//! ## Error Checking
//!
//! After each command, the driver queries `ERR?` to check for errors. The controller
//! returns `0` for success or an error code (1-25+) for various failure conditions.
//!
//! # Transport
//!
//! This implementation uses TCP/IP on port 50000 (the E-727's default).
//!
//! **Note:** USB transport was attempted but abandoned due to firmware bugs in the
//! E-727 that cause the USB bulk IN endpoint to stop responding after reading
//! 2-3 short packets (<64 bytes). The TCP/IP transport does not have this issue
//! since it uses streaming sockets without packet boundaries.
//!
//! # Character Encoding
//!
//! PI devices use Latin-1 (CP1252) encoding, not UTF-8. This is particularly
//! relevant for the `PUN?` (physical unit) command which returns characters
//! like `µ` (micro sign, 0xB5) for units like `µrad`. The driver automatically
//! converts Latin-1 responses to UTF-8 strings.
//!
//! # References
//!
//! - [PI E-727 Documentation (Google Drive)](https://drive.google.com/drive/u/0/folders/1ebFyabBYmZ5Ts942VnFBqXl_U1nlaOlV)

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use thiserror::Error;
use tracing::debug;

use super::e727::PiErrorCode;

/// Default TCP port for PI controllers.
///
/// The E-727 listens on port 50000 by default, configurable via the device's
/// network settings (parameters 0x11000800+).
pub const DEFAULT_PORT: u16 = 50000;

/// Default timeout for operations (matches Python driver's 7 second default).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(7);

/// Errors that can occur during GCS communication.
///
/// These errors cover connection issues, protocol errors, and controller-reported
/// errors. Use the `ControllerError` variant's `code` field to identify specific
/// PI error conditions.
#[derive(Error, Debug)]
pub enum GcsError {
    /// Low-level I/O error (socket read/write failure).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to establish TCP connection to the controller.
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    /// No response received within the timeout period.
    #[error("Timeout waiting for response")]
    Timeout,

    /// Response from controller doesn't match expected format.
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Controller reported an error via `ERR?` query.
    #[error("Controller error {code}: {}", error.map(|e| e.description()).unwrap_or("Unknown error"))]
    ControllerError {
        /// Raw error code from controller
        code: i32,
        /// Decoded error if known
        error: Option<PiErrorCode>,
    },

    /// Failed to parse response values.
    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Result type for GCS operations.
pub type GcsResult<T> = Result<T, GcsError>;

/// Low-level GCS device communicating over TCP/IP.
///
/// This struct handles the TCP communication with PI controllers,
/// implementing the GCS 2.0 protocol message framing. It provides:
///
/// - Connection management via [`connect`](Self::connect)
/// - Raw command sending via [`send`](Self::send)
/// - Response reading with GCS EOL detection via [`read`](Self::read)
/// - Convenience methods for queries and commands with error checking
/// - Response parsing utilities for axis=value formats
///
/// For high-level FSM control with typed methods, use [`E727`](super::E727) instead.
pub struct GcsDevice {
    stream: TcpStream,
    timeout: Duration,
    address: String,
}

impl GcsDevice {
    /// Connect to a PI controller at the given address.
    ///
    /// # Arguments
    ///
    /// * `addr` - Socket address to connect to. Can be:
    ///   - `"192.168.15.210:50000"` - IP with explicit port
    ///   - `"hostname:50000"` - Hostname with port
    ///   - Any type implementing [`ToSocketAddrs`]
    ///
    /// # Errors
    ///
    /// Returns [`GcsError::ConnectionFailed`] if the TCP connection cannot be established.
    pub fn connect<A: ToSocketAddrs + ToString>(addr: A) -> GcsResult<Self> {
        let address = addr.to_string();
        let stream = TcpStream::connect(&addr)
            .map_err(|e| GcsError::ConnectionFailed(format!("Failed to connect: {e}")))?;

        stream.set_read_timeout(Some(DEFAULT_TIMEOUT))?;
        stream.set_write_timeout(Some(DEFAULT_TIMEOUT))?;

        debug!("Connected to PI device via TCP");

        let mut device = Self {
            stream,
            timeout: DEFAULT_TIMEOUT,
            address,
        };

        // Flush any stale data from previous aborted connections
        device.flush_buffers();

        Ok(device)
    }

    /// Reconnect to the controller using the stored address.
    ///
    /// Use this to recover from connection errors or socket timeouts.
    pub fn reconnect(&mut self) -> GcsResult<()> {
        debug!("Reconnecting to {}", self.address);
        let stream = TcpStream::connect(&self.address)
            .map_err(|e| GcsError::ConnectionFailed(format!("Failed to reconnect: {e}")))?;

        stream.set_read_timeout(Some(self.timeout))?;
        stream.set_write_timeout(Some(self.timeout))?;

        self.stream = stream;

        // Flush any stale data from previous aborted connections
        self.flush_buffers();

        debug!("Reconnected to PI device");
        Ok(())
    }

    /// Connect to a PI controller at the given IP using the default port (50000).
    ///
    /// This is a convenience method equivalent to `connect("ip:50000")`.
    pub fn connect_default_port(ip: &str) -> GcsResult<Self> {
        Self::connect(format!("{ip}:{DEFAULT_PORT}"))
    }

    /// Set the timeout for read/write operations.
    ///
    /// The default timeout is 7 seconds (matching the PI Python driver).
    /// Some operations like homing or long moves may require longer timeouts.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
        let _ = self.stream.set_read_timeout(Some(timeout));
        let _ = self.stream.set_write_timeout(Some(timeout));
    }

    /// Flush any stale data from buffers after connection.
    ///
    /// This handles the case where a previous connection was aborted mid-communication,
    /// leaving residual data in the controller's buffers. We:
    /// 1. Send an empty line to terminate any partial command the controller was waiting for
    /// 2. Drain any pending response data with a short timeout
    /// 3. Clear any error state by reading ERR?
    fn flush_buffers(&mut self) {
        // Use a short timeout for flushing
        let _ = self
            .stream
            .set_read_timeout(Some(Duration::from_millis(100)));

        // Send empty line to terminate any partial command
        let _ = self.stream.write_all(b"\n");
        let _ = self.stream.flush();

        // Drain any pending data (ignore errors/timeouts)
        let mut buf = [0u8; 1024];
        loop {
            match self.stream.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => continue,
                Err(_) => break,
            }
        }

        // Clear error state
        let _ = self.stream.write_all(b"ERR?\n");
        let _ = self.stream.flush();

        // Drain error response
        loop {
            match self.stream.read(&mut buf) {
                Ok(0) => break,
                Ok(_) => continue,
                Err(_) => break,
            }
        }

        // Restore normal timeout
        let _ = self.stream.set_read_timeout(Some(self.timeout));
    }

    /// Send a raw command string to the device.
    ///
    /// Appends a newline if not present. Does not wait for or read any response.
    /// Use [`read`](Self::read) to get the response, or use [`query`](Self::query)
    /// for commands that return data.
    pub fn send(&mut self, command: &str) -> GcsResult<()> {
        let mut msg = command.to_string();
        if !msg.ends_with('\n') {
            msg.push('\n');
        }

        debug!("GCS send: {:?}", msg.trim());
        self.stream.write_all(msg.as_bytes())?;
        self.stream.flush()?;
        Ok(())
    }

    /// Read a complete GCS response from the device.
    ///
    /// Reads bytes until detecting the GCS end-of-line marker: a newline (`\n`)
    /// that is NOT preceded by a space. Multi-line responses use ` \n` (space+newline)
    /// as line separators.
    ///
    /// # Character Encoding
    ///
    /// PI devices use Latin-1 (CP1252) encoding. This method automatically converts
    /// the response to a UTF-8 string, correctly handling characters like `µ` (0xB5).
    ///
    /// # Errors
    ///
    /// Returns [`GcsError::Timeout`] if no complete response is received within
    /// the configured timeout period.
    pub fn read(&mut self) -> GcsResult<String> {
        let mut buf = [0u8; 1];
        let mut bytes = Vec::new();

        loop {
            match self.stream.read(&mut buf) {
                Ok(0) => return Err(GcsError::Timeout),
                Ok(_) => {
                    bytes.push(buf[0]);

                    // Check for GCS EOL: '\n' not preceded by space
                    if buf[0] == b'\n' {
                        let len = bytes.len();
                        if len < 2 || bytes[len - 2] != b' ' {
                            break;
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    debug!("Read timeout TimedOut");
                    return Err(GcsError::Timeout);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    debug!("Read timeout WouldBlock");
                    return Err(GcsError::Timeout);
                }
                Err(e) => {
                    debug!("Read error: {}", e);
                    return Err(e.into());
                }
            }
        }

        // Convert Latin-1 to UTF-8 (Latin-1 bytes 0-255 map directly to Unicode code points)
        let response: String = bytes.iter().map(|&b| b as char).collect();
        debug!("GCS recv: {:?}", response);
        Ok(response)
    }

    /// Send a query command and read the response with automatic error checking.
    ///
    /// After receiving the response, this method sends `ERR?` to the controller
    /// and verifies no error occurred. This matches the behavior of the official
    /// PI Python driver.
    ///
    /// # Errors
    ///
    /// Returns [`GcsError::ControllerError`] if the controller reports an error
    /// after processing the command.
    pub fn query(&mut self, command: &str) -> GcsResult<String> {
        self.send(command)?;
        let response = self.read()?;

        // Check for errors after each query
        self.send("ERR?")?;
        let err_response = self.read()?;
        let err_code: i32 = err_response.trim().parse().unwrap_or(0);
        if err_code != 0 {
            return Err(GcsError::ControllerError {
                code: err_code,
                error: PiErrorCode::from_code(err_code),
            });
        }
        Ok(response)
    }

    /// Send a command and check for errors (for commands with no response).
    ///
    /// Use this for commands like `MOV`, `SVO`, etc. that don't return data
    /// but should be checked for errors.
    pub fn command(&mut self, command: &str) -> GcsResult<()> {
        self.send(command)?;
        self.check_error()
    }

    /// Check for controller errors by querying `ERR?`.
    ///
    /// Returns `Ok(())` if no error (code 0), otherwise returns the error.
    pub fn check_error(&mut self) -> GcsResult<()> {
        self.send("ERR?")?;
        let response = self.read()?;
        let error_code: i32 = response
            .trim()
            .parse()
            .map_err(|_| GcsError::InvalidResponse(format!("Invalid error code: {response}")))?;

        if error_code != 0 {
            Err(GcsError::ControllerError {
                code: error_code,
                error: PiErrorCode::from_code(error_code),
            })
        } else {
            Ok(())
        }
    }

    /// Parse `axis=value` response format into a HashMap.
    ///
    /// Handles multi-line responses where each line is `axis=value`.
    pub fn parse_axis_values(response: &str) -> GcsResult<HashMap<String, f64>> {
        let mut result = HashMap::new();

        for line in response.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() != 2 {
                return Err(GcsError::ParseError(format!(
                    "Invalid response format: {line}"
                )));
            }

            let axis = parts[0].trim().to_string();
            let value: f64 = parts[1].trim().parse().map_err(|_| {
                GcsError::ParseError(format!("Invalid number: {}", parts[1].trim()))
            })?;

            result.insert(axis, value);
        }

        Ok(result)
    }

    /// Parse a single `axis=value` response and return just the value.
    ///
    /// Useful for queries like `POS? 1` that return a single axis value.
    pub fn parse_single_value(response: &str) -> GcsResult<f64> {
        let values = Self::parse_axis_values(response)?;
        values
            .into_values()
            .next()
            .ok_or_else(|| GcsError::ParseError("No value in response".to_string()))
    }

    /// Parse `axis=bool` response format into a HashMap.
    ///
    /// Interprets `0` as false and `1` or `true` as true.
    pub fn parse_axis_bools(response: &str) -> GcsResult<HashMap<String, bool>> {
        let mut result = HashMap::new();

        for line in response.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() != 2 {
                return Err(GcsError::ParseError(format!(
                    "Invalid response format: {line}"
                )));
            }

            let axis = parts[0].trim().to_string();
            let value = parts[1].trim();
            let bool_val = value == "1" || value.eq_ignore_ascii_case("true");

            result.insert(axis, bool_val);
        }

        Ok(result)
    }
}
