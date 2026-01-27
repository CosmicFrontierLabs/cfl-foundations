//! FSM interface trait for calibration workflows.

/// Interface for FSM control
///
/// Abstracts the FSM hardware for testability in calibration workflows.
pub trait FsmInterface {
    /// Send a command to move both FSM axes
    ///
    /// # Arguments
    /// * `axis1_urad` - Axis 1 command in microradians
    /// * `axis2_urad` - Axis 2 command in microradians
    fn move_to(&mut self, axis1_urad: f64, axis2_urad: f64) -> Result<(), String>;

    /// Get the current FSM position
    fn get_position(&mut self) -> Result<(f64, f64), String>;
}
