//! PI E-727 Digital Multi-Channel Piezo Controller Driver
//!
//! This module provides a high-level interface to the PI E-727 controller,
//! commonly used with fast steering mirrors (FSM) for optical beam pointing.
//!
//! # Overview
//!
//! The E-727 is a digital piezo controller supporting up to 4 axes of closed-loop
//! servo control. This driver exposes the most commonly used commands for FSM operation:
//!
//! - **Position control**: [`move_to`](E727::move_to), [`move_relative`](E727::move_relative),
//!   [`get_position`](E727::get_position)
//! - **Servo control**: [`set_servo`](E727::set_servo), [`get_servo`](E727::get_servo)
//! - **Motion queries**: [`is_on_target`](E727::is_on_target), [`wait_on_target`](E727::wait_on_target)
//! - **Emergency stop**: [`stop_all`](E727::stop_all), [`halt`](E727::halt)
//!
//! # Axis Configuration
//!
//! The E-727 typically controls a 2-axis fast steering mirror with additional
//! axes for focus or other adjustments:
//!
//! - **Axis 1, 2**: Tilt axes (typically 0-2000 µrad range)
//! - **Axis 3**: Additional tilt or unused (±2500 µrad range on some configs)
//! - **Axis 4**: Piston/focus axis (typically 0-100 µm range)
//!
//! Query axis configuration with [`axes()`](E727::axes) and [`get_travel_range()`](E727::get_travel_range).
//!
//! # Servo Control
//!
//! The E-727 uses closed-loop servo control for precision positioning. Before
//! moving, you must enable the servo on each axis:
//!
//! ```no_run
//! use hardware::pi::E727;
//!
//! let mut fsm = E727::connect_ip("192.168.15.210")?;
//!
//! // Enable servos (required before motion commands)
//! fsm.set_all_servos(true)?;
//!
//! // Now moves will work
//! fsm.move_to("1", 1000.0)?;
//! # Ok::<(), hardware::pi::GcsError>(())
//! ```
//!
//! # Connection
//!
//! The E-727 is connected via TCP/IP on port 50000. The default IP is
//! 192.168.168.10, but DHCP is enabled so the device may acquire a different
//! address on your network.
//!
//! **Note:** USB transport was attempted but has firmware bugs causing
//! communication failures after 2-3 short packet reads.
//!
//! # Example: Complete FSM Control
//!
//! ```no_run
//! use hardware::pi::E727;
//! use std::time::Duration;
//!
//! // Connect to E-727
//! let mut fsm = E727::connect_ip("192.168.15.210")?;
//!
//! // Check device identity
//! println!("Connected to: {}", fsm.idn()?);
//! println!("Axes: {:?}", fsm.axes());
//!
//! // Query current state
//! for axis in fsm.axes().to_vec() {
//!     let (min, max) = fsm.get_travel_range(&axis)?;
//!     let unit = fsm.get_unit(&axis)?;
//!     let pos = fsm.get_position(&axis)?;
//!     println!("Axis {}: {:.1} {} (range {:.1}-{:.1})", axis, pos, unit, min, max);
//! }
//!
//! // Enable servos and move
//! fsm.set_all_servos(true)?;
//! fsm.move_to("1", 1000.0)?;
//! fsm.move_to("2", 1000.0)?;
//!
//! // Wait for motion to complete
//! fsm.wait_on_target(Duration::from_secs(5))?;
//!
//! // Check final positions
//! let positions = fsm.get_all_positions()?;
//! println!("Final positions: {:?}", positions);
//!
//! # Ok::<(), hardware::pi::GcsError>(())
//! ```
//!
//! # Safety
//!
//! The E-727 has built-in position limits, but care should still be taken:
//!
//! - Always check [`get_travel_range()`](E727::get_travel_range) before commanding large moves
//! - Use [`stop_all()`](E727::stop_all) for emergency stops (sends Ctrl+X)
//! - Disable servos when not actively controlling the mirror
//!
//! # References
//!
//! - E-727 User Manual: `ext_ref/E727-UserManual.txt`
//! - GCS Commands: `ext_ref/GCS-Commands.txt`
//! - Python reference: `ext_ref/PIPython/.../pipython/pidevice/gcs2/gcs2commands.py`

use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::time::Duration;

use tracing::{debug, info};

use super::gcs::{GcsDevice, GcsError, GcsResult, DEFAULT_PORT};

/// High-level driver for the PI E-727 digital piezo controller.
///
/// Provides convenient methods for fast steering mirror control including
/// position commands, servo control, and motion monitoring.
///
/// # Example
///
/// ```no_run
/// use hardware::pi::E727;
/// use std::time::Duration;
///
/// let mut fsm = E727::connect_ip("192.168.15.210")?;
/// fsm.set_all_servos(true)?;
/// fsm.move_to("1", 1000.0)?;
/// fsm.wait_on_target(Duration::from_secs(2))?;
/// # Ok::<(), hardware::pi::GcsError>(())
/// ```
pub struct E727 {
    device: GcsDevice,
    axes: Vec<String>,
}

impl E727 {
    /// Connect to an E-727 at the given address.
    ///
    /// # Arguments
    ///
    /// * `addr` - Socket address (IP:port). For just IP, use [`connect_ip`](Self::connect_ip).
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hardware::pi::E727;
    ///
    /// let mut fsm = E727::connect("192.168.15.210:50000")?;
    /// # Ok::<(), hardware::pi::GcsError>(())
    /// ```
    pub fn connect<A: ToSocketAddrs>(addr: A) -> GcsResult<Self> {
        let device = GcsDevice::connect(addr)?;
        Self::init(device)
    }

    /// Connect to an E-727 at the given IP using the default port (50000).
    ///
    /// This is the recommended connection method for most use cases.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use hardware::pi::E727;
    ///
    /// let mut fsm = E727::connect_ip("192.168.15.210")?;
    /// println!("Connected to: {}", fsm.idn()?);
    /// # Ok::<(), hardware::pi::GcsError>(())
    /// ```
    pub fn connect_ip(ip: &str) -> GcsResult<Self> {
        let device = GcsDevice::connect(format!("{ip}:{DEFAULT_PORT}"))?;
        Self::init(device)
    }

    /// Initialize the E727 driver after connection.
    fn init(mut device: GcsDevice) -> GcsResult<Self> {
        let idn = device.query("*IDN?")?;
        info!("Connected to: {}", idn.trim());

        let axes = Self::query_axes(&mut device)?;
        debug!("Available axes: {:?}", axes);

        Ok(Self { device, axes })
    }

    /// Query available axes from the controller.
    fn query_axes(device: &mut GcsDevice) -> GcsResult<Vec<String>> {
        let response = device.query("SAI?")?;
        Ok(response
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect())
    }

    /// Set the timeout for operations.
    ///
    /// The default is 7 seconds. Increase for long moves or homing operations.
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.device.set_timeout(timeout);
    }

    /// Query device identification string.
    ///
    /// Returns a string like:
    /// `(c)2015 Physik Instrumente (PI) GmbH & Co. KG, E-727, 0116044408, 13.21.00.09`
    pub fn idn(&mut self) -> GcsResult<String> {
        let response = self.device.query("*IDN?")?;
        Ok(response.trim().to_string())
    }

    /// Get the list of available axis identifiers.
    ///
    /// Typically returns `["1", "2", "3", "4"]` for a 4-axis E-727.
    pub fn axes(&self) -> &[String] {
        &self.axes
    }

    // ==================== Position Queries ====================

    /// Get current position of an axis in physical units (µrad or µm).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use hardware::pi::E727;
    /// # let mut fsm = E727::connect_ip("192.168.15.210")?;
    /// let pos = fsm.get_position("1")?;
    /// println!("Axis 1 position: {} µrad", pos);
    /// # Ok::<(), hardware::pi::GcsError>(())
    /// ```
    pub fn get_position(&mut self, axis: &str) -> GcsResult<f64> {
        let response = self.device.query(&format!("POS? {axis}"))?;
        GcsDevice::parse_single_value(&response)
    }

    /// Get current positions of all axes.
    ///
    /// Returns a HashMap mapping axis ID to position value.
    pub fn get_all_positions(&mut self) -> GcsResult<HashMap<String, f64>> {
        let response = self.device.query("POS?")?;
        GcsDevice::parse_axis_values(&response)
    }

    /// Get target (commanded) position of an axis.
    ///
    /// This is the position commanded by the last `MOV` command, which may
    /// differ from the actual position if motion is in progress.
    pub fn get_target(&mut self, axis: &str) -> GcsResult<f64> {
        let response = self.device.query(&format!("MOV? {axis}"))?;
        GcsDevice::parse_single_value(&response)
    }

    /// Get target positions of all axes.
    pub fn get_all_targets(&mut self) -> GcsResult<HashMap<String, f64>> {
        let response = self.device.query("MOV?")?;
        GcsDevice::parse_axis_values(&response)
    }

    // ==================== Motion Commands ====================

    /// Move an axis to an absolute position.
    ///
    /// The servo must be enabled on the axis before motion commands will work.
    /// The command returns immediately; use [`wait_on_target`](Self::wait_on_target)
    /// to wait for motion completion.
    ///
    /// # Arguments
    ///
    /// * `axis` - Axis identifier (e.g., "1", "2")
    /// * `position` - Target position in physical units (µrad or µm)
    ///
    /// # Errors
    ///
    /// Returns `ControllerError` with code 7 if position is out of limits,
    /// or code 5 if servo is not enabled.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use hardware::pi::E727;
    /// # let mut fsm = E727::connect_ip("192.168.15.210")?;
    /// fsm.set_servo("1", true)?;
    /// fsm.move_to("1", 1000.0)?;  // Move to 1000 µrad
    /// # Ok::<(), hardware::pi::GcsError>(())
    /// ```
    pub fn move_to(&mut self, axis: &str, position: f64) -> GcsResult<()> {
        self.device.command(&format!("MOV {axis} {position}"))
    }

    /// Move multiple axes to absolute positions simultaneously.
    ///
    /// This sends a single command to move all specified axes at once,
    /// which is more efficient than multiple [`move_to`](Self::move_to) calls.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use hardware::pi::E727;
    /// # let mut fsm = E727::connect_ip("192.168.15.210")?;
    /// fsm.set_all_servos(true)?;
    /// fsm.move_all(&[("1", 1000.0), ("2", 1000.0)])?;
    /// # Ok::<(), hardware::pi::GcsError>(())
    /// ```
    pub fn move_all(&mut self, positions: &[(impl AsRef<str>, f64)]) -> GcsResult<()> {
        let args: Vec<String> = positions
            .iter()
            .map(|(axis, pos)| format!("{} {}", axis.as_ref(), pos))
            .collect();
        self.device.command(&format!("MOV {}", args.join(" ")))
    }

    /// Move an axis by a relative distance.
    ///
    /// # Arguments
    ///
    /// * `axis` - Axis identifier
    /// * `distance` - Distance to move (positive or negative)
    pub fn move_relative(&mut self, axis: &str, distance: f64) -> GcsResult<()> {
        self.device.command(&format!("MVR {axis} {distance}"))
    }

    // ==================== Servo Control ====================

    /// Enable or disable servo (closed-loop) control for an axis.
    ///
    /// Servo must be enabled before motion commands will work. When disabled,
    /// the piezo operates in open-loop mode.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use hardware::pi::E727;
    /// # let mut fsm = E727::connect_ip("192.168.15.210")?;
    /// fsm.set_servo("1", true)?;   // Enable servo
    /// fsm.set_servo("1", false)?;  // Disable servo
    /// # Ok::<(), hardware::pi::GcsError>(())
    /// ```
    pub fn set_servo(&mut self, axis: &str, enabled: bool) -> GcsResult<()> {
        let state = if enabled { 1 } else { 0 };
        self.device.command(&format!("SVO {axis} {state}"))
    }

    /// Enable or disable servo control for all axes.
    ///
    /// Convenience method to set all axes at once.
    pub fn set_all_servos(&mut self, enabled: bool) -> GcsResult<()> {
        for axis in self.axes.clone() {
            self.set_servo(&axis, enabled)?;
        }
        Ok(())
    }

    /// Get servo (closed-loop) state for an axis.
    ///
    /// Returns `true` if servo is enabled, `false` if disabled (open-loop).
    pub fn get_servo(&mut self, axis: &str) -> GcsResult<bool> {
        let response = self.device.query(&format!("SVO? {axis}"))?;
        let values = GcsDevice::parse_axis_bools(&response)?;
        values
            .into_values()
            .next()
            .ok_or_else(|| GcsError::ParseError("No servo state in response".to_string()))
    }

    /// Get servo states for all axes.
    pub fn get_all_servos(&mut self) -> GcsResult<HashMap<String, bool>> {
        let response = self.device.query("SVO?")?;
        GcsDevice::parse_axis_bools(&response)
    }

    // ==================== Motion Status ====================

    /// Check if an axis is on target (motion complete).
    ///
    /// Returns `true` if the axis has reached its commanded position within
    /// the configured settling window.
    pub fn is_on_target(&mut self, axis: &str) -> GcsResult<bool> {
        let response = self.device.query(&format!("ONT? {axis}"))?;
        let values = GcsDevice::parse_axis_bools(&response)?;
        values
            .into_values()
            .next()
            .ok_or_else(|| GcsError::ParseError("No on-target state in response".to_string()))
    }

    /// Get on-target state for all axes.
    pub fn all_on_target(&mut self) -> GcsResult<HashMap<String, bool>> {
        let response = self.device.query("ONT?")?;
        GcsDevice::parse_axis_bools(&response)
    }

    /// Check if any axis is currently moving (via control byte query).
    ///
    /// This uses the special `\x05` control byte command.
    ///
    /// **Note:** This command may return unexpected data over TCP. Prefer
    /// checking [`all_on_target()`](Self::all_on_target) instead.
    pub fn is_moving(&mut self) -> GcsResult<bool> {
        self.device.send("\x05")?;
        let response = self.device.read()?;
        let status: u8 = response
            .trim()
            .parse()
            .map_err(|_| GcsError::InvalidResponse(format!("Invalid motion status: {response}")))?;
        Ok(status != 0)
    }

    /// Check if controller is ready (via control byte query).
    ///
    /// This uses the special `\x07` control byte command.
    ///
    /// **Note:** This command may return unexpected data over TCP. Prefer
    /// checking [`all_on_target()`](Self::all_on_target) instead.
    pub fn is_ready(&mut self) -> GcsResult<bool> {
        self.device.send("\x07")?;
        let response = self.device.read()?;
        let byte = response.bytes().next().unwrap_or(0);
        Ok(byte == 0xB1)
    }

    // ==================== Emergency Stop ====================

    /// Emergency stop all axes (sends Ctrl+X).
    ///
    /// This immediately stops all motion. Use in emergency situations or
    /// when you need to abort motion quickly.
    pub fn stop_all(&mut self) -> GcsResult<()> {
        self.device.send("\x18")?;
        Ok(())
    }

    /// Halt a specific axis.
    ///
    /// Stops motion on the specified axis while leaving other axes unaffected.
    pub fn halt(&mut self, axis: &str) -> GcsResult<()> {
        self.device.command(&format!("HLT {axis}"))
    }

    // ==================== Configuration Queries ====================

    /// Get the travel range limits for an axis.
    ///
    /// Returns `(min, max)` position values in physical units (µrad or µm).
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use hardware::pi::E727;
    /// # let mut fsm = E727::connect_ip("192.168.15.210")?;
    /// let (min, max) = fsm.get_travel_range("1")?;
    /// println!("Axis 1 range: {} to {} µrad", min, max);
    /// # Ok::<(), hardware::pi::GcsError>(())
    /// ```
    pub fn get_travel_range(&mut self, axis: &str) -> GcsResult<(f64, f64)> {
        let min_response = self.device.query(&format!("TMN? {axis}"))?;
        let max_response = self.device.query(&format!("TMX? {axis}"))?;
        let min = GcsDevice::parse_single_value(&min_response)?;
        let max = GcsDevice::parse_single_value(&max_response)?;
        Ok((min, max))
    }

    /// Get the physical unit for an axis.
    ///
    /// Returns strings like `"µrad"` (microradians) or `"µm"` (micrometers).
    pub fn get_unit(&mut self, axis: &str) -> GcsResult<String> {
        let response = self.device.query(&format!("PUN? {axis}"))?;
        for line in response.lines() {
            if let Some((_axis, value)) = line.split_once('=') {
                return Ok(value.trim().to_string());
            }
        }
        Err(GcsError::ParseError("No unit in response".to_string()))
    }

    // ==================== Utility Methods ====================

    /// Wait until all axes are on target or timeout.
    ///
    /// Polls [`all_on_target()`](Self::all_on_target) every 10ms until all axes
    /// report being on target, or the timeout expires.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use hardware::pi::E727;
    /// use std::time::Duration;
    ///
    /// # let mut fsm = E727::connect_ip("192.168.15.210")?;
    /// fsm.set_all_servos(true)?;
    /// fsm.move_to("1", 1000.0)?;
    /// fsm.wait_on_target(Duration::from_secs(5))?;
    /// println!("Motion complete!");
    /// # Ok::<(), hardware::pi::GcsError>(())
    /// ```
    pub fn wait_on_target(&mut self, timeout: Duration) -> GcsResult<()> {
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > timeout {
                return Err(GcsError::Timeout);
            }

            let on_target = self.all_on_target()?;
            if on_target.values().all(|&v| v) {
                return Ok(());
            }

            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Query the last error code from the controller.
    ///
    /// Returns `0` if no error, otherwise a PI error code.
    /// See [`GcsError::ControllerError`] for common error codes.
    pub fn last_error(&mut self) -> GcsResult<i32> {
        self.device.send("ERR?")?;
        let response = self.device.read()?;
        response
            .trim()
            .parse()
            .map_err(|_| GcsError::InvalidResponse(format!("Invalid error code: {response}")))
    }
}
