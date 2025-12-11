//! Data recorder types and configuration for the E-727.
//!
//! The E-727 includes a real-time data recorder that can capture various signals
//! at up to 50 kHz. This module provides typed enums for configuring what data
//! to record and what triggers the recording.

use super::Axis;

/// Trigger source for data recording.
///
/// Determines when the data recorder starts capturing data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordTrigger {
    /// Recording triggered by IMP, STE, WGO, or WGR commands.
    ///
    /// This is the default mode where recording only starts when
    /// explicitly triggered by impulse/step response or wave generator commands.
    Default,

    /// Recording triggered when any motion command is sent.
    ///
    /// Triggers on MOV, MVR, SVA, or SVR commands. Useful for capturing
    /// the transient response to position commands.
    OnMove,

    /// Recording triggered by external digital input line.
    ///
    /// The value specifies which input line (1-4), or 0 for any line.
    ExternalInput(u8),

    /// Recording starts immediately when configured.
    ///
    /// The DRT command itself triggers recording to begin.
    Immediate,
}

impl RecordTrigger {
    /// Convert to DRT command arguments (trigger_source, value).
    pub(crate) fn to_drt_args(self) -> (u8, u8) {
        match self {
            RecordTrigger::Default => (0, 0),
            RecordTrigger::OnMove => (1, 0),
            RecordTrigger::ExternalInput(line) => (3, line),
            RecordTrigger::Immediate => (4, 0),
        }
    }
}

/// What data to record from the E-727.
///
/// Each variant specifies both the data source and the record option.
/// The E-727 can record up to 8 channels simultaneously.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordChannel {
    // ==================== Axis-based options ====================
    /// Target position of axis (corresponds to MOV? response).
    TargetPosition(Axis),

    /// Current position of axis (corresponds to POS? response).
    CurrentPosition(Axis),

    /// Position error of axis (target - current).
    PositionError(Axis),

    /// DDL (Dynamic Digital Linearization) output of axis.
    DdlOutput(Axis),

    /// Open loop control value (corresponds to SVA? response).
    OpenLoopControl(Axis),

    /// Control output before axis-to-output transformation.
    ControlOutput(Axis),

    /// Slowed target position (after slew rate limiting).
    SlowedTarget(Axis),

    // ==================== Output signal channel options ====================
    /// Control voltage of output signal channel (1-4).
    ///
    /// Value after axis-to-output transformation but before output type definition.
    ControlVoltage(u8),

    /// Output voltage of signal channel (corresponds to VOL? response).
    ///
    /// Value after axis-to-output transformation and output type definition.
    OutputVoltage(u8),

    // ==================== Input signal channel options ====================
    /// Normalized sensor value (corresponds to TNS? response).
    SensorNormalized(u8),

    /// Sensor value after filtering.
    AfterFiltering(u8),

    /// Sensor value after electronics linearization.
    AfterLinearization(u8),

    /// Sensor value after mechanics linearization (corresponds to TSP? response).
    AfterMechanicsLinearization(u8),

    // ==================== Digital I/O ====================
    /// Digital input values (binary coded: In1*1 + In2*2 + In3*4 + In4*8).
    DigitalInput,

    /// Digital output values (binary coded: Out1*2 + Out2*4 + Out3*8).
    DigitalOutput,
}

impl RecordChannel {
    /// Convert to DRC command arguments (source, record_option).
    pub(crate) fn to_drc_args(self) -> (String, u8) {
        match self {
            // Axis-based options
            RecordChannel::TargetPosition(axis) => (axis.to_string(), 1),
            RecordChannel::CurrentPosition(axis) => (axis.to_string(), 2),
            RecordChannel::PositionError(axis) => (axis.to_string(), 3),
            RecordChannel::DdlOutput(axis) => (axis.to_string(), 13),
            RecordChannel::OpenLoopControl(axis) => (axis.to_string(), 14),
            RecordChannel::ControlOutput(axis) => (axis.to_string(), 15),
            RecordChannel::SlowedTarget(axis) => (axis.to_string(), 22),

            // Output signal channel options
            RecordChannel::ControlVoltage(ch) => (ch.to_string(), 7),
            RecordChannel::OutputVoltage(ch) => (ch.to_string(), 16),

            // Input signal channel options
            RecordChannel::SensorNormalized(ch) => (ch.to_string(), 17),
            RecordChannel::AfterFiltering(ch) => (ch.to_string(), 18),
            RecordChannel::AfterLinearization(ch) => (ch.to_string(), 19),
            RecordChannel::AfterMechanicsLinearization(ch) => (ch.to_string(), 20),

            // Digital I/O (source is dummy 0)
            RecordChannel::DigitalInput => ("0".to_string(), 26),
            RecordChannel::DigitalOutput => ("0".to_string(), 27),
        }
    }
}
