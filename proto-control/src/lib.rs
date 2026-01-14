#![doc = include_str!("../README.md")]

/// Timestamp in microseconds since the initialization of the control loop.
///
/// Maximum representable time: ~584,942 years. If your mission exceeds this
/// duration, congratulations on the interstellar voyage and/or achieving
/// functional immortality. Please file a bug report from Alpha Centauri.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Timestamp(pub u64);

impl Timestamp {
    /// Create timestamp from microseconds.
    pub fn from_micros(micros: u64) -> Self {
        Self(micros)
    }

    /// Get timestamp as microseconds.
    pub fn as_micros(&self) -> u64 {
        self.0
    }
}

/// Gyro tick counter at 500Hz.
///
/// Represents discrete timing ticks synchronized with gyroscope measurements.
/// Maximum representable time: ~99.42 days (2^32 ticks at 500Hz)
///
/// # Behavior
///
/// - Increments by exactly 1 for each `GyroReadout` result
/// - Strictly monotonic (always increasing, never decreases or repeats)
/// - Provides consistent time reference across all sensor measurements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GyroTick(pub u32);

/// Gyroscope readout representing integrated angle on three axes.
///
/// All angular values are in radians, representing the integrated angle
/// as reported by the Exail gyroscope hardware.
///
/// # Timing
///
/// The `timestamp` field represents the time difference between the current
/// XYZ angle measurement and the previous measurement, as reported by the
/// Exail gyroscope hardware. The timestamp is aligned to the first moment
/// the measurement was computed by the gyro.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GyroReadout {
    /// X-axis angle in radians
    pub x: f64,
    /// Y-axis angle in radians
    pub y: f64,
    /// Z-axis angle in radians
    pub z: f64,
    /// Timestamp representing time difference from previous measurement,
    /// aligned to the first moment the measurement was computed by the gyro
    pub timestamp: Timestamp,
}

impl GyroReadout {
    /// Create new gyro readout with angles in radians.
    pub fn new(x: f64, y: f64, z: f64, timestamp: Timestamp) -> Self {
        Self { x, y, z, timestamp }
    }

    /// Get angles as an array [x, y, z] in radians.
    pub fn as_array(&self) -> [f64; 3] {
        [self.x, self.y, self.z]
    }

    /// Convert angles to arcseconds.
    pub fn to_arcseconds(&self) -> [f64; 3] {
        const RAD_TO_ARCSEC: f64 = 206264.80624709636;
        [
            self.x * RAD_TO_ARCSEC,
            self.y * RAD_TO_ARCSEC,
            self.z * RAD_TO_ARCSEC,
        ]
    }
}

/// Fine Guidance System 2D angular estimate with uncertainty.
///
/// Represents pointing direction in two angular dimensions with
/// variance estimates for each axis.
///
/// # Timing
///
/// The `timestamp` field corresponds to the center of the image exposure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FgsReadout {
    /// X-axis angular position in arcseconds
    pub x: f64,
    /// Y-axis angular position in arcseconds
    pub y: f64,
    /// Variance of x-axis measurement in arcseconds²
    pub x_variance: f64,
    /// Variance of y-axis measurement in arcseconds²
    pub y_variance: f64,
    /// Timestamp corresponding to center of image exposure
    pub timestamp: Timestamp,
}

impl FgsReadout {
    /// Create new FGS readout.
    pub fn new(x: f64, y: f64, x_variance: f64, y_variance: f64, timestamp: Timestamp) -> Self {
        Self {
            x,
            y,
            x_variance,
            y_variance,
            timestamp,
        }
    }

    /// Convert angular positions to radians.
    pub fn to_radians(&self) -> [f64; 2] {
        const ARCSEC_TO_RAD: f64 = 4.84813681109536e-6;
        [self.x * ARCSEC_TO_RAD, self.y * ARCSEC_TO_RAD]
    }

    /// Get variances in radians².
    pub fn variance_radians(&self) -> [f64; 2] {
        const ARCSEC_TO_RAD: f64 = 4.84813681109536e-6;
        const ARCSEC2_TO_RAD2: f64 = ARCSEC_TO_RAD * ARCSEC_TO_RAD;
        [
            self.x_variance * ARCSEC2_TO_RAD2,
            self.y_variance * ARCSEC2_TO_RAD2,
        ]
    }

    /// Get standard deviations in arcseconds.
    pub fn std_dev(&self) -> [f64; 2] {
        [self.x_variance.sqrt(), self.y_variance.sqrt()]
    }
}

/// FSM (Fast Steering Mirror) readout with X and Y axis feedback.
///
/// Represents voltage feedback from the fast steering mirror on two orthogonal axes.
///
/// # Timing
///
/// The `timestamp` field is intended to indicate the center of the ADC
/// (Analog-to-Digital Converter) window from the ExoLambda board.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FsmReadout {
    /// X-axis voltage readout in volts
    pub vx: f64,
    /// Y-axis voltage readout in volts
    pub vy: f64,
    /// Timestamp indicating center of ADC window from ExoLambda board
    pub timestamp: Timestamp,
}

impl FsmReadout {
    /// Create new FSM readout.
    pub fn new(vx: f64, vy: f64, timestamp: Timestamp) -> Self {
        Self { vx, vy, timestamp }
    }
}

/// FSM (Fast Steering Mirror) command with X and Y axis voltages.
///
/// Represents commanded voltages for the fast steering mirror on two orthogonal axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FsmCommand {
    /// X-axis voltage command in volts
    pub vx: f64,
    /// Y-axis voltage command in volts
    pub vy: f64,
}

impl FsmCommand {
    /// Create new FSM command.
    pub fn new(vx: f64, vy: f64) -> Self {
        Self { vx, vy }
    }
}

/// Estimator state containing sensor measurements and control outputs.
///
/// Represents the complete state used by the estimation and control system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EstimatorState {
    /// Gyro tick for this state
    pub gyro_tick: GyroTick,
    /// Gyroscope readout
    pub gyro: GyroReadout,
    /// FSM voltage readout
    pub fsm_readout: FsmReadout,
    /// Optional fine guidance system readout
    pub fgs_readout: Option<FgsReadout>,
    /// FSM voltage command
    pub fsm_command: FsmCommand,
}

impl EstimatorState {
    /// Create new estimator state.
    pub fn new(
        gyro_tick: GyroTick,
        gyro: GyroReadout,
        fsm_readout: FsmReadout,
        fgs_readout: Option<FgsReadout>,
        fsm_command: FsmCommand,
    ) -> Self {
        Self {
            gyro_tick,
            gyro,
            fsm_readout,
            fgs_readout,
            fsm_command,
        }
    }
}

/// Trait for line-of-sight state estimator implementation.
///
/// Implement this trait to provide state estimation and control logic
/// for the LOS control algorithm. The estimator processes sensor measurements
/// and state history to compute the next FSM command.
///
/// # Function Signature (Conceptual)
///
/// ```text
/// f: (&[EstimatorState], &GyroReadout, &FsmReadout, Option<&FgsReadout>) → FsmCommand
/// ```
///
/// # State History
///
/// The `state_history` parameter contains previous `EstimatorState` values
/// assembled by the caller from prior calls. This vector provides the estimator
/// with access to historical sensor readings and commands for filtering,
/// prediction, and state estimation purposes.
///
/// **Ordering Requirements:**
///
/// - Elements are ordered chronologically: `state_history[0]` is the oldest state
/// - `state_history[n-1]` is the most recent state (where n = length)
/// - On the first call, `state_history` will be empty (`&[]`)
///
/// **Length Constraints:**
///
/// - The vector will retain a fixed maximum number of previous states
/// - When the maximum length is reached, oldest states are removed as new ones are added
/// - The exact maximum length is implementation-defined (FIFO buffer behavior)
/// - The estimator should not assume any specific history length
///
/// # Parameters
///
/// - `state_history`: Previous estimator states assembled by caller (ordered oldest to newest)
/// - `gyro_readout`: Current gyroscope angle readout
/// - `fsm_readout`: Current FSM voltage readout
/// - `fgs_readout`: Optional fine guidance system readout. None if FGS
///   data is not available for this cycle.
///
/// # Returns
///
/// The computed FSM voltage command for this cycle.
///
/// # Caller Responsibilities
///
/// Upon return of the `FsmCommand`, it is the caller's responsibility to:
///
/// - Assemble an `EstimatorState` from the passed sensor readings and returned command
/// - Append the new state to the history for subsequent calls
/// - Perform the `FsmCommand` adjustment to the FSM hardware at the appropriate time
/// - Push a Line-of-Sight (LOS) update to the payload computer at the required interval
pub trait StateEstimator {
    /// Compute FSM command from sensor measurements and history.
    ///
    /// # Parameters
    ///
    /// - `state_history`: Previous estimator states assembled by caller,
    ///   ordered oldest to newest (index 0 = oldest). Empty on first call.
    ///   See trait-level documentation for detailed ordering requirements.
    /// - `gyro_readout`: Current gyroscope angle readout
    /// - `fsm_readout`: Current FSM voltage readout
    /// - `fgs_readout`: Optional fine guidance system update. None if FGS
    ///   update is not available for this cycle. When present, represents
    ///   a new fine guidance measurement.
    ///
    /// # Returns
    ///
    /// The computed FSM voltage command for this cycle.
    fn estimate(
        &self,
        state_history: &[EstimatorState],
        gyro_readout: &GyroReadout,
        fsm_readout: &FsmReadout,
        fgs_readout: Option<&FgsReadout>,
    ) -> FsmCommand;
}
