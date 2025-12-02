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
//! # Example
//!
//! ```no_run
//! use hardware::pi::GcsDevice;
//!
//! // Connect to controller
//! let mut device = GcsDevice::connect("192.168.15.210:50000")?;
//!
//! // Query device identification
//! let idn = device.query("*IDN?")?;
//! println!("Device: {}", idn.trim());
//!
//! // Query position of axis 1
//! let response = device.query("POS? 1")?;
//! let pos = GcsDevice::parse_single_value(&response)?;
//! println!("Position: {} µrad", pos);
//!
//! // Send a move command (no response expected)
//! device.command("MOV 1 1000.0")?;
//!
//! # Ok::<(), hardware::pi::GcsError>(())
//! ```
//!
//! # References
//!
//! - GCS Commands Manual: `ext_ref/GCS-Commands.txt` (converted from PZ281E PDF)
//! - Python reference: `ext_ref/PIPython/PIPython/extracted/PIPython-2.10.2.1/pipython/`
//!   - `pidevice/interfaces/pisocket.py` - TCP transport
//!   - `pidevice/gcsmessages.py` - Message framing and error checking
//!   - `pidevice/gcs2/gcs2commands.py` - High-level command wrappers

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use thiserror::Error;
use tracing::{debug, trace};

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
    ///
    /// Common error codes:
    /// - 1: Parameter syntax error
    /// - 2: Unknown command
    /// - 5: Unallowable move on unreferenced axis
    /// - 6: Parameter out of range
    /// - 7: Position out of limits
    /// - 10: Controller in wrong state
    /// - 23: Invalid axis identifier
    #[error("Controller error {code}: {message}")]
    ControllerError {
        /// PI error code (1-25+)
        code: i32,
        /// Human-readable error description
        message: String,
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
///
/// # Example
///
/// ```no_run
/// use hardware::pi::GcsDevice;
///
/// let mut device = GcsDevice::connect_default_port("192.168.15.210")?;
///
/// // Low-level: send command and read response separately
/// device.send("POS?")?;
/// let response = device.read()?;
///
/// // Higher-level: query with automatic error checking
/// let response = device.query("SVO?")?;
///
/// // Parse the response
/// let servo_states = GcsDevice::parse_axis_bools(&response)?;
/// for (axis, enabled) in servo_states {
///     println!("Axis {}: servo={}", axis, enabled);
/// }
///
/// # Ok::<(), hardware::pi::GcsError>(())
/// ```
pub struct GcsDevice {
    stream: TcpStream,
    timeout: Duration,
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
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hardware::pi::GcsDevice;
    ///
    /// let mut device = GcsDevice::connect("192.168.15.210:50000")?;
    /// let idn = device.query("*IDN?")?;
    /// println!("Connected to: {}", idn.trim());
    /// # Ok::<(), hardware::pi::GcsError>(())
    /// ```
    pub fn connect<A: ToSocketAddrs>(addr: A) -> GcsResult<Self> {
        let stream = TcpStream::connect(&addr)
            .map_err(|e| GcsError::ConnectionFailed(format!("Failed to connect: {e}")))?;

        stream.set_read_timeout(Some(DEFAULT_TIMEOUT))?;
        stream.set_write_timeout(Some(DEFAULT_TIMEOUT))?;

        debug!("Connected to PI device via TCP");

        Ok(Self {
            stream,
            timeout: DEFAULT_TIMEOUT,
        })
    }

    /// Connect to a PI controller at the given IP using the default port (50000).
    ///
    /// This is a convenience method equivalent to `connect("ip:50000")`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hardware::pi::GcsDevice;
    ///
    /// let mut device = GcsDevice::connect_default_port("192.168.15.210")?;
    /// # Ok::<(), hardware::pi::GcsError>(())
    /// ```
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

    /// Send a raw command string to the device.
    ///
    /// Appends a newline if not present. Does not wait for or read any response.
    /// Use [`read`](Self::read) to get the response, or use [`query`](Self::query)
    /// for commands that return data.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use hardware::pi::GcsDevice;
    /// # let mut device = GcsDevice::connect_default_port("192.168.15.210")?;
    /// // Send emergency stop (no response expected)
    /// device.send("\x18")?;
    /// # Ok::<(), hardware::pi::GcsError>(())
    /// ```
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
                    return Err(GcsError::Timeout);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    return Err(GcsError::Timeout);
                }
                Err(e) => return Err(e.into()),
            }
        }

        // Convert Latin-1 to UTF-8 (Latin-1 bytes 0-255 map directly to Unicode code points)
        let response: String = bytes.iter().map(|&b| b as char).collect();
        trace!("GCS recv: {:?}", response);
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
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use hardware::pi::GcsDevice;
    /// # let mut device = GcsDevice::connect_default_port("192.168.15.210")?;
    /// let response = device.query("POS? 1")?;
    /// let position = GcsDevice::parse_single_value(&response)?;
    /// println!("Axis 1 position: {}", position);
    /// # Ok::<(), hardware::pi::GcsError>(())
    /// ```
    pub fn query(&mut self, command: &str) -> GcsResult<String> {
        self.send(command)?;
        let response = self.read()?;

        // Check for errors after each query
        self.send("ERR?")?;
        let err_response = self.read()?;
        let err_code: i32 = err_response.trim().parse().unwrap_or(-1);
        if err_code != 0 {
            return Err(GcsError::ControllerError {
                code: err_code,
                message: Self::error_message(err_code),
            });
        }
        Ok(response)
    }

    /// Send a command and check for errors (for commands with no response).
    ///
    /// Use this for commands like `MOV`, `SVO`, etc. that don't return data
    /// but should be checked for errors.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use hardware::pi::GcsDevice;
    /// # let mut device = GcsDevice::connect_default_port("192.168.15.210")?;
    /// // Enable servo on axis 1
    /// device.command("SVO 1 1")?;
    ///
    /// // Move axis 1 to position 1000
    /// device.command("MOV 1 1000.0")?;
    /// # Ok::<(), hardware::pi::GcsError>(())
    /// ```
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
                message: Self::error_message(error_code),
            })
        } else {
            Ok(())
        }
    }

    /// Get human-readable message for a PI error code.
    fn error_message(code: i32) -> String {
        match code {
            1 => "Parameter syntax error".to_string(),
            2 => "Unknown command".to_string(),
            3 => "Command length out of limits".to_string(),
            4 => "Error while scanning".to_string(),
            5 => "Unallowable move attempted on unreferenced axis".to_string(),
            6 => "Parameter out of range".to_string(),
            7 => "Position out of limits".to_string(),
            10 => "Controller in wrong state".to_string(),
            17 => "Param not found in non-volatile memory".to_string(),
            23 => "Invalid axis identifier".to_string(),
            24 => "Incorrect number of parameters".to_string(),
            25 => "Invalid floating point number".to_string(),
            _ => format!("Unknown error ({code})"),
        }
    }

    /// Parse `axis=value` response format into a HashMap.
    ///
    /// Handles multi-line responses like:
    /// ```text
    /// 1=100.5
    /// 2=200.3
    /// ```
    ///
    /// # Example
    ///
    /// ```
    /// use hardware::pi::GcsDevice;
    ///
    /// let response = "1=100.5 \n2=200.3\n";
    /// let values = GcsDevice::parse_axis_values(response).unwrap();
    /// assert_eq!(values.get("1"), Some(&100.5));
    /// assert_eq!(values.get("2"), Some(&200.3));
    /// ```
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
    ///
    /// # Example
    ///
    /// ```
    /// use hardware::pi::GcsDevice;
    ///
    /// let response = "1=1234.5\n";
    /// let value = GcsDevice::parse_single_value(response).unwrap();
    /// assert_eq!(value, 1234.5);
    /// ```
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
    ///
    /// # Example
    ///
    /// ```
    /// use hardware::pi::GcsDevice;
    ///
    /// let response = "1=1 \n2=0\n";
    /// let values = GcsDevice::parse_axis_bools(response).unwrap();
    /// assert_eq!(values.get("1"), Some(&true));
    /// assert_eq!(values.get("2"), Some(&false));
    /// ```
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
