use clap::ValueEnum;

/// E-727 axis identifiers (1-based).
///
/// The E-727 supports up to 4 axes, numbered 1-4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Axis {
    /// Axis 1 (typically X tilt)
    Axis1 = 1,
    /// Axis 2 (typically Y tilt)
    Axis2 = 2,
    /// Axis 3 (additional tilt or unused)
    Axis3 = 3,
    /// Axis 4 (typically piston/focus)
    Axis4 = 4,
}

impl Axis {
    /// Get the 1-based axis number.
    pub fn number(self) -> u8 {
        self as u8
    }

    /// Get the axis identifier string for GCS commands.
    pub fn as_str(self) -> &'static str {
        match self {
            Axis::Axis1 => "1",
            Axis::Axis2 => "2",
            Axis::Axis3 => "3",
            Axis::Axis4 => "4",
        }
    }
}

impl std::fmt::Display for Axis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.number())
    }
}

impl std::str::FromStr for Axis {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "1" => Ok(Axis::Axis1),
            "2" => Ok(Axis::Axis2),
            "3" => Ok(Axis::Axis3),
            "4" => Ok(Axis::Axis4),
            _ => Err(format!("Invalid axis: {s}, expected 1-4")),
        }
    }
}

/// E-727 SPA (Set Parameter Access) parameter IDs.
///
/// These hex addresses are used with SPA/SPA? commands to read/write
/// controller parameters. Use `HPA?` command to list all available parameters.
///
/// # HPA? Response Format
/// The HPA? command returns parameter info in this format:
/// ```text
/// <PamID>=<CmdLevel>TAB<MaxItem>TAB<DataType>TAB<FuncGroup>TAB<Description>
/// ```
/// Where:
/// - `CmdLevel`: Command level required for write access (1-3)
/// - `MaxItem`: Number of items this parameter applies to:
///   - 1 = System-wide (query without axis: `SPA? 0xADDR`)
///   - 4 = Per-axis (query per axis: `SPA? 1 0xADDR`)
///   - 7 = Per input channel
///   - 8 = Per recorder table
/// - `DataType`: INT, FLOAT, or CHAR
/// - `FuncGroup`: Functional group (Servo, System, Output, etc.)
/// - `Description`: Human-readable parameter name
#[derive(Debug, Clone, Copy, ValueEnum, strum::EnumIter, strum::Display)]
#[repr(u32)]
#[allow(clippy::upper_case_acronyms)]
pub enum SpaParam {
    // ==================== System (0x02) ====================
    #[value(name = "sensor-interface-type")]
    SensorInterfaceType = 0x02000001,
    #[value(name = "sensor-range-factor")]
    SensorRangeFactor = 0x02000100,
    #[value(name = "sensor-offset-factor")]
    SensorOffsetFactor = 0x02000102,
    #[value(name = "sensor-mech-corr-1")]
    SensorMechCorrection1 = 0x02000200,
    #[value(name = "sensor-mech-corr-2")]
    SensorMechCorrection2 = 0x02000300,
    #[value(name = "sensor-mech-corr-3")]
    SensorMechCorrection3 = 0x02000400,
    #[value(name = "sensor-mech-corr-4")]
    SensorMechCorrection4 = 0x02000500,
    #[value(name = "sensor-mech-corr-5")]
    SensorMechCorrection5 = 0x02000600,
    #[value(name = "sensor-ref-mode")]
    SensorReferenceMode = 0x02000A00,
    #[value(name = "lim-ref-signals")]
    LimRefSignalsDetectable = 0x02001900,

    // ==================== Input/Sensor Elec. Corrections (0x03) ====================
    // Channel 1 (19 coefficients)
    #[value(name = "elec-corr-1-1")]
    SensorElecCorr1_1 = 0x03000100,
    #[value(name = "elec-corr-1-2")]
    SensorElecCorr1_2 = 0x03000101,
    #[value(name = "elec-corr-1-3")]
    SensorElecCorr1_3 = 0x03000102,
    #[value(name = "elec-corr-1-4")]
    SensorElecCorr1_4 = 0x03000103,
    #[value(name = "elec-corr-1-5")]
    SensorElecCorr1_5 = 0x03000104,
    #[value(name = "elec-corr-1-6")]
    SensorElecCorr1_6 = 0x03000105,
    #[value(name = "elec-corr-1-7")]
    SensorElecCorr1_7 = 0x03000106,
    #[value(name = "elec-corr-1-8")]
    SensorElecCorr1_8 = 0x03000107,
    #[value(name = "elec-corr-1-9")]
    SensorElecCorr1_9 = 0x03000108,
    #[value(name = "elec-corr-1-10")]
    SensorElecCorr1_10 = 0x03000109,
    #[value(name = "elec-corr-1-11")]
    SensorElecCorr1_11 = 0x0300010A,
    #[value(name = "elec-corr-1-12")]
    SensorElecCorr1_12 = 0x0300010B,
    #[value(name = "elec-corr-1-13")]
    SensorElecCorr1_13 = 0x0300010C,
    #[value(name = "elec-corr-1-14")]
    SensorElecCorr1_14 = 0x0300010D,
    #[value(name = "elec-corr-1-15")]
    SensorElecCorr1_15 = 0x0300010E,
    #[value(name = "elec-corr-1-16")]
    SensorElecCorr1_16 = 0x0300010F,
    #[value(name = "elec-corr-1-17")]
    SensorElecCorr1_17 = 0x03000110,
    #[value(name = "elec-corr-1-18")]
    SensorElecCorr1_18 = 0x03000111,
    #[value(name = "elec-corr-1-19")]
    SensorElecCorr1_19 = 0x03000112,

    // Channel 2 (19 coefficients)
    #[value(name = "elec-corr-2-1")]
    SensorElecCorr2_1 = 0x03000200,
    #[value(name = "elec-corr-2-2")]
    SensorElecCorr2_2 = 0x03000201,
    #[value(name = "elec-corr-2-3")]
    SensorElecCorr2_3 = 0x03000202,
    #[value(name = "elec-corr-2-4")]
    SensorElecCorr2_4 = 0x03000203,
    #[value(name = "elec-corr-2-5")]
    SensorElecCorr2_5 = 0x03000204,
    #[value(name = "elec-corr-2-6")]
    SensorElecCorr2_6 = 0x03000205,
    #[value(name = "elec-corr-2-7")]
    SensorElecCorr2_7 = 0x03000206,
    #[value(name = "elec-corr-2-8")]
    SensorElecCorr2_8 = 0x03000207,
    #[value(name = "elec-corr-2-9")]
    SensorElecCorr2_9 = 0x03000208,
    #[value(name = "elec-corr-2-10")]
    SensorElecCorr2_10 = 0x03000209,
    #[value(name = "elec-corr-2-11")]
    SensorElecCorr2_11 = 0x0300020A,
    #[value(name = "elec-corr-2-12")]
    SensorElecCorr2_12 = 0x0300020B,
    #[value(name = "elec-corr-2-13")]
    SensorElecCorr2_13 = 0x0300020C,
    #[value(name = "elec-corr-2-14")]
    SensorElecCorr2_14 = 0x0300020D,
    #[value(name = "elec-corr-2-15")]
    SensorElecCorr2_15 = 0x0300020E,
    #[value(name = "elec-corr-2-16")]
    SensorElecCorr2_16 = 0x0300020F,
    #[value(name = "elec-corr-2-17")]
    SensorElecCorr2_17 = 0x03000210,
    #[value(name = "elec-corr-2-18")]
    SensorElecCorr2_18 = 0x03000211,
    #[value(name = "elec-corr-2-19")]
    SensorElecCorr2_19 = 0x03000212,

    // Channel 3 (19 coefficients)
    #[value(name = "elec-corr-3-1")]
    SensorElecCorr3_1 = 0x03000300,
    #[value(name = "elec-corr-3-2")]
    SensorElecCorr3_2 = 0x03000301,
    #[value(name = "elec-corr-3-3")]
    SensorElecCorr3_3 = 0x03000302,
    #[value(name = "elec-corr-3-4")]
    SensorElecCorr3_4 = 0x03000303,
    #[value(name = "elec-corr-3-5")]
    SensorElecCorr3_5 = 0x03000304,
    #[value(name = "elec-corr-3-6")]
    SensorElecCorr3_6 = 0x03000305,
    #[value(name = "elec-corr-3-7")]
    SensorElecCorr3_7 = 0x03000306,
    #[value(name = "elec-corr-3-8")]
    SensorElecCorr3_8 = 0x03000307,
    #[value(name = "elec-corr-3-9")]
    SensorElecCorr3_9 = 0x03000308,
    #[value(name = "elec-corr-3-10")]
    SensorElecCorr3_10 = 0x03000309,
    #[value(name = "elec-corr-3-11")]
    SensorElecCorr3_11 = 0x0300030A,
    #[value(name = "elec-corr-3-12")]
    SensorElecCorr3_12 = 0x0300030B,
    #[value(name = "elec-corr-3-13")]
    SensorElecCorr3_13 = 0x0300030C,
    #[value(name = "elec-corr-3-14")]
    SensorElecCorr3_14 = 0x0300030D,
    #[value(name = "elec-corr-3-15")]
    SensorElecCorr3_15 = 0x0300030E,
    #[value(name = "elec-corr-3-16")]
    SensorElecCorr3_16 = 0x0300030F,
    #[value(name = "elec-corr-3-17")]
    SensorElecCorr3_17 = 0x03000310,
    #[value(name = "elec-corr-3-18")]
    SensorElecCorr3_18 = 0x03000311,
    #[value(name = "elec-corr-3-19")]
    SensorElecCorr3_19 = 0x03000312,

    // Channel 4 (19 coefficients)
    #[value(name = "elec-corr-4-1")]
    SensorElecCorr4_1 = 0x03000400,
    #[value(name = "elec-corr-4-2")]
    SensorElecCorr4_2 = 0x03000401,
    #[value(name = "elec-corr-4-3")]
    SensorElecCorr4_3 = 0x03000402,
    #[value(name = "elec-corr-4-4")]
    SensorElecCorr4_4 = 0x03000403,
    #[value(name = "elec-corr-4-5")]
    SensorElecCorr4_5 = 0x03000404,
    #[value(name = "elec-corr-4-6")]
    SensorElecCorr4_6 = 0x03000405,
    #[value(name = "elec-corr-4-7")]
    SensorElecCorr4_7 = 0x03000406,
    #[value(name = "elec-corr-4-8")]
    SensorElecCorr4_8 = 0x03000407,
    #[value(name = "elec-corr-4-9")]
    SensorElecCorr4_9 = 0x03000408,
    #[value(name = "elec-corr-4-10")]
    SensorElecCorr4_10 = 0x03000409,
    #[value(name = "elec-corr-4-11")]
    SensorElecCorr4_11 = 0x0300040A,
    #[value(name = "elec-corr-4-12")]
    SensorElecCorr4_12 = 0x0300040B,
    #[value(name = "elec-corr-4-13")]
    SensorElecCorr4_13 = 0x0300040C,
    #[value(name = "elec-corr-4-14")]
    SensorElecCorr4_14 = 0x0300040D,
    #[value(name = "elec-corr-4-15")]
    SensorElecCorr4_15 = 0x0300040E,
    #[value(name = "elec-corr-4-16")]
    SensorElecCorr4_16 = 0x0300040F,
    #[value(name = "elec-corr-4-17")]
    SensorElecCorr4_17 = 0x03000410,
    #[value(name = "elec-corr-4-18")]
    SensorElecCorr4_18 = 0x03000411,
    #[value(name = "elec-corr-4-19")]
    SensorElecCorr4_19 = 0x03000412,

    // Channel 5 (19 coefficients)
    #[value(name = "elec-corr-5-1")]
    SensorElecCorr5_1 = 0x03000500,
    #[value(name = "elec-corr-5-2")]
    SensorElecCorr5_2 = 0x03000501,
    #[value(name = "elec-corr-5-3")]
    SensorElecCorr5_3 = 0x03000502,
    #[value(name = "elec-corr-5-4")]
    SensorElecCorr5_4 = 0x03000503,
    #[value(name = "elec-corr-5-5")]
    SensorElecCorr5_5 = 0x03000504,
    #[value(name = "elec-corr-5-6")]
    SensorElecCorr5_6 = 0x03000505,
    #[value(name = "elec-corr-5-7")]
    SensorElecCorr5_7 = 0x03000506,
    #[value(name = "elec-corr-5-8")]
    SensorElecCorr5_8 = 0x03000507,
    #[value(name = "elec-corr-5-9")]
    SensorElecCorr5_9 = 0x03000508,
    #[value(name = "elec-corr-5-10")]
    SensorElecCorr5_10 = 0x03000509,
    #[value(name = "elec-corr-5-11")]
    SensorElecCorr5_11 = 0x0300050A,
    #[value(name = "elec-corr-5-12")]
    SensorElecCorr5_12 = 0x0300050B,
    #[value(name = "elec-corr-5-13")]
    SensorElecCorr5_13 = 0x0300050C,
    #[value(name = "elec-corr-5-14")]
    SensorElecCorr5_14 = 0x0300050D,
    #[value(name = "elec-corr-5-15")]
    SensorElecCorr5_15 = 0x0300050E,
    #[value(name = "elec-corr-5-16")]
    SensorElecCorr5_16 = 0x0300050F,
    #[value(name = "elec-corr-5-17")]
    SensorElecCorr5_17 = 0x03000510,
    #[value(name = "elec-corr-5-18")]
    SensorElecCorr5_18 = 0x03000511,
    #[value(name = "elec-corr-5-19")]
    SensorElecCorr5_19 = 0x03000512,

    // ==================== Input/Sensor Offset Corrections ====================
    #[value(name = "sensor-offset-corr-1")]
    SensorOffsetCorrection1 = 0x03001000,
    #[value(name = "sensor-offset-corr-2")]
    SensorOffsetCorrection2 = 0x03001100,
    #[value(name = "sensor-offset-corr-3")]
    SensorOffsetCorrection3 = 0x03001200,
    #[value(name = "sensor-offset-corr-4")]
    SensorOffsetCorrection4 = 0x03001300,
    #[value(name = "sensor-offset-corr-5")]
    SensorOffsetCorrection5 = 0x03001400,
    #[value(name = "sensor-offset-corr-6")]
    SensorOffsetCorrection6 = 0x03001500,
    #[value(name = "input-num-format")]
    InputNumericalFormat = 0x03003400,
    #[value(name = "sensor-autoscale-enable")]
    SensorAutoscalingEnable = 0x03003700,
    #[value(name = "sensor-autoscale-gain")]
    SensorAutoscalingGain = 0x03003701,
    #[value(name = "cbi")]
    CBI = 0x03003800,
    #[value(name = "sbi")]
    SBI = 0x03003801,
    #[value(name = "oai")]
    OAI = 0x03003803,

    // ==================== Digital Filter (0x05) ====================
    #[value(name = "digital-filter-type")]
    DigitalFilterType = 0x05000000,
    #[value(name = "digital-filter-bw")]
    DigitalFilterBandwidth = 0x05000001,
    #[value(name = "user-filter-1")]
    UserFilterParam1 = 0x05000101,
    #[value(name = "user-filter-2")]
    UserFilterParam2 = 0x05000102,
    #[value(name = "user-filter-3")]
    UserFilterParam3 = 0x05000103,
    #[value(name = "user-filter-4")]
    UserFilterParam4 = 0x05000104,
    #[value(name = "user-filter-5")]
    UserFilterParam5 = 0x05000105,

    // ==================== Analog Target (0x06) ====================
    #[value(name = "adc-channel-target")]
    ADCChannelForTarget = 0x06000500,
    #[value(name = "analog-target-offset")]
    AnalogTargetOffset = 0x06000501,

    // ==================== Servo (0x07) ====================
    #[value(name = "range-min")]
    RangeLimitMin = 0x07000000,
    #[value(name = "range-max")]
    RangeLimitMax = 0x07000001,
    #[value(name = "slew-rate")]
    SlewRate = 0x07000200,
    #[value(name = "slew-rate-open")]
    SlewRateOpen = 0x07000201,
    #[value(name = "p-term")]
    PTerm = 0x07000300,
    #[value(name = "i-term")]
    ITerm = 0x07000301,
    #[value(name = "d-term")]
    DTerm = 0x07000302,
    #[value(name = "p-term-velocity")]
    PTermVelocity = 0x07000307,
    #[value(name = "i-term-velocity")]
    ITermVelocity = 0x07000308,
    #[value(name = "d-term-velocity")]
    DTermVelocity = 0x07000309,
    #[value(name = "pos-from-sensor-1")]
    PositionFromSensor1 = 0x07000500,
    #[value(name = "pos-from-sensor-2")]
    PositionFromSensor2 = 0x07000501,
    #[value(name = "pos-from-sensor-3")]
    PositionFromSensor3 = 0x07000502,
    #[value(name = "pos-from-sensor-4")]
    PositionFromSensor4 = 0x07000503,
    #[value(name = "pos-from-sensor-5")]
    PositionFromSensor5 = 0x07000504,
    #[value(name = "pos-from-sensor-6")]
    PositionFromSensor6 = 0x07000505,
    #[value(name = "pos-from-sensor-7")]
    PositionFromSensor7 = 0x07000506,
    #[value(name = "axis-name")]
    AxisName = 0x07000600,
    #[value(name = "axis-unit")]
    AxisUnit = 0x07000601,
    #[value(name = "powerup-servo")]
    PowerUpServoEnable = 0x07000800,
    #[value(name = "powerup-atz")]
    PowerUpAutoZeroEnable = 0x07000802,
    #[value(name = "on-target-tol")]
    OnTargetTolerance = 0x07000900,
    #[value(name = "settling-time")]
    SettlingTime = 0x07000901,
    #[value(name = "atz-low-v")]
    AutoZeroLowVoltage = 0x07000A00,
    #[value(name = "atz-high-v")]
    AutoZeroHighVoltage = 0x07000A01,
    #[value(name = "default-voltage")]
    DefaultVoltage = 0x07000C01,
    #[value(name = "pos-report-scale")]
    PositionReportScaling = 0x07001005,
    #[value(name = "pos-report-offset")]
    PositionReportOffset = 0x07001006,
    #[value(name = "open-loop-mode")]
    OpenLoopControlMode = 0x07022000,
    #[value(name = "closed-loop-mode")]
    ClosedLoopControlMode = 0x07030100,
    #[value(name = "ref-velocity")]
    ReferencingVelocity = 0x07030300,
    #[value(name = "ff-gain")]
    FeedForwardGain = 0x07030600,
    #[value(name = "ff-input-channel")]
    FeedForwardInputChannel = 0x07030900,
    #[value(name = "on-target-fix-i")]
    OnTargetToleranceFixITerm = 0x07030A00,
    #[value(name = "hlt-slow-down")]
    HLTSlowDown = 0x07030B00,
    #[value(name = "id-chip-axis-map")]
    IDChipAxisMapCtrl = 0x07030C00,
    #[value(name = "zero-if-i-fixed")]
    FlagToZeroIfITermFixed = 0x07030D00,

    // ==================== Notch Filters (0x08) ====================
    #[value(name = "notch-freq-1")]
    NotchFreq1 = 0x08000100,
    #[value(name = "notch-freq-2")]
    NotchFreq2 = 0x08000101,
    #[value(name = "notch-reject-1")]
    NotchReject1 = 0x08000200,
    #[value(name = "notch-reject-2")]
    NotchReject2 = 0x08000201,
    #[value(name = "notch-bw-1")]
    NotchBandwidth1 = 0x08000300,
    #[value(name = "notch-bw-2")]
    NotchBandwidth2 = 0x08000301,
    #[value(name = "creep-t1")]
    CreepT1 = 0x08000400,
    #[value(name = "creep-t2")]
    CreepT2 = 0x08000401,
    #[value(name = "notch-open-loop")]
    NotchInOpenLoop = 0x08000500,
    #[value(name = "notch-calc-method")]
    NotchFilterCalcMethod = 0x08000600,

    // ==================== Piezo Driving (0x09) ====================
    #[value(name = "piezo-1")]
    DrivingFactorPiezo1 = 0x09000000,
    #[value(name = "piezo-2")]
    DrivingFactorPiezo2 = 0x09000001,
    #[value(name = "piezo-3")]
    DrivingFactorPiezo3 = 0x09000002,
    #[value(name = "piezo-4")]
    DrivingFactorPiezo4 = 0x09000003,

    // ==================== Output Selection (0x0A) ====================
    #[value(name = "output-type")]
    SelectOutputType = 0x0A000003,
    #[value(name = "output-index")]
    SelectOutputIndex = 0x0A000004,

    // ==================== Output/Amplifier (0x0B) ====================
    #[value(name = "amp-v-min")]
    AmpVoltageMin = 0x0B000007,
    #[value(name = "amp-v-max")]
    AmpVoltageMax = 0x0B000008,
    #[value(name = "amp-v-zero-dac")]
    AmpVoltageZeroDac = 0x0B000009,
    #[value(name = "amp-v-offset")]
    AmpVoltageOffset = 0x0B00000A,
    #[value(name = "output-num-format")]
    OutputNumericalFormat = 0x0B000500,
    #[value(name = "input-index-feedback")]
    InputIndexToFeedBack = 0x0B000800,
    #[value(name = "amp2-v-min")]
    Amp2VoltageMin = 0x0B000A00,
    #[value(name = "amp2-v-max")]
    Amp2VoltageMax = 0x0B000A01,
    #[value(name = "amp2-v-zero-dac")]
    Amp2VoltageZeroDac = 0x0B000A02,

    // ==================== Soft Voltage Limits (0x0C) ====================
    #[value(name = "soft-v-low")]
    SoftVoltageLowLimit = 0x0C000000,
    #[value(name = "soft-v-high")]
    SoftVoltageHighLimit = 0x0C000001,

    // ==================== Device Info (0x0D) ====================
    #[value(name = "device-sn")]
    DeviceSerialNumber = 0x0D000000,
    #[value(name = "hw-sn")]
    HardwareSerialNumber = 0x0D000100,
    #[value(name = "hw-name")]
    HardwareName = 0x0D000200,

    // ==================== System Timing/Config (0x0E) ====================
    #[value(name = "sensor-sample-time")]
    SensorSamplingTime = 0x0E000100,
    #[value(name = "servo-update-time")]
    ServoUpdateTime = 0x0E000200,
    #[value(name = "disable-error-10")]
    DisableError10 = 0x0E000301,
    #[value(name = "ddl-license")]
    DDLLicense = 0x0E000400,
    #[value(name = "ddl-license-valid")]
    DDLLicenseValid = 0x0E000401,
    #[value(name = "num-input-channels")]
    NumInputChannels = 0x0E000B00,
    #[value(name = "num-output-channels")]
    NumOutputChannels = 0x0E000B01,
    #[value(name = "num-system-axes")]
    NumSystemAxes = 0x0E000B02,
    #[value(name = "num-sensor-channels")]
    NumSensorChannels = 0x0E000B03,
    #[value(name = "num-piezo-channels")]
    NumPiezoChannels = 0x0E000B04,
    #[value(name = "num-trigger-outputs")]
    NumTriggerOutputs = 0x0E000B05,
    #[value(name = "num-piezowalk-channels")]
    NumPiezoWalkChannels = 0x0E000B06,
    #[value(name = "num-conf-piezowalk")]
    NumConfPiezoWalkChannels = 0x0E000B07,
    #[value(name = "oversample-filter")]
    OverSamplingFilter = 0x0E000C01,
    #[value(name = "adv-piezo-license")]
    AdvPiezoControlLicense = 0x0E000E00,
    #[value(name = "adv-piezo-license-valid")]
    AdvPiezoControlLicenseValid = 0x0E000F00,
    #[value(name = "reboot-on-dio")]
    RebootOnDIOInput = 0x0E001500,
    #[value(name = "trigger-input-filter")]
    TriggerInputFilterEnable = 0x0E001D00,
    #[value(name = "discon-target-stop")]
    DisconTargetManInWithStop = 0x0E001E00,
    #[value(name = "check-sensor-plausibility")]
    CheckSensorPositionPlausibility = 0x0E001F00,
    #[value(name = "move-to-last-cmd-pos")]
    MoveToLastCommandedPosition = 0x0E002000,

    // ==================== ID-Chip/Stage (0x0F) ====================
    #[value(name = "powerup-read-id")]
    PowerUpReadIDChip = 0x0F000000,
    #[value(name = "stage-type")]
    StageType = 0x0F000100,
    #[value(name = "stage-sn")]
    StageSerialNumber = 0x0F000200,
    #[value(name = "stage-assembly-date")]
    StageAssemblyDate = 0x0F000300,

    // ==================== FastIF (0x10) ====================
    #[value(name = "fastif-axis-input")]
    FastIFAxisInputUsage = 0x10000500,
    #[value(name = "fastif-data-type")]
    FastIFDataType = 0x10000501,
    #[value(name = "fastif-data-low")]
    FastIFDataLowLimit = 0x10000502,
    #[value(name = "fastif-data-high")]
    FastIFDataHighLimit = 0x10000503,
    #[value(name = "fastif-used-low")]
    FastIFUsedLowLimit = 0x10000504,
    #[value(name = "fastif-used-high")]
    FastIFUsedHighLimit = 0x10000505,
    #[value(name = "fastif-used-range")]
    FastIFUsedRange = 0x10000506,
    #[value(name = "fastif-ol-low")]
    FastIFOpenLoopLowLimit = 0x10000507,
    #[value(name = "fastif-ol-high")]
    FastIFOpenLoopHighLimit = 0x10000508,
    #[value(name = "fastif-ol-extend")]
    FastIFOpenLoopLimitExtend = 0x10000509,

    // ==================== StdIF/Network (0x11) ====================
    #[value(name = "uart-baudrate")]
    UartBaudrate = 0x11000400,
    #[value(name = "ip-address")]
    IPAddress = 0x11000600,
    #[value(name = "ip-mask")]
    IPMask = 0x11000700,
    #[value(name = "ip-config")]
    IPConfiguration = 0x11000800,
    #[value(name = "mac-address")]
    MACAddress = 0x11000B00,

    // ==================== Wave Generator (0x13) ====================
    #[value(name = "max-wave-points")]
    MaxWavePoints = 0x13000004,
    #[value(name = "wave-table-rate")]
    WaveGeneratorTableRate = 0x13000109,
    #[value(name = "num-waves")]
    NumWaves = 0x1300010A,
    #[value(name = "wave-offset")]
    WaveOffset = 0x1300010B,
    #[value(name = "wave-multi-start-trig")]
    WaveMultiStartByTrigger = 0x13000202,

    // ==================== DDL (0x14) ====================
    #[value(name = "ddl-repeat")]
    DDLRepeatNumber = 0x14000001,
    #[value(name = "ddl-delay-max")]
    DDLTimeDelayMax = 0x14000006,
    #[value(name = "ddl-delay-min")]
    DDLTimeDelayMin = 0x14000007,
    #[value(name = "ddl-delay-rule")]
    DDLTimeDelayChangeRule = 0x14000008,
    #[value(name = "ddl-zero-gain")]
    DDLZeroGainNumber = 0x1400000A,
    #[value(name = "ddl-max-points")]
    DDLMaxPoints = 0x1400000B,

    // ==================== Data Recorder (0x16) ====================
    #[value(name = "recorder-table-rate")]
    RecorderTableRate = 0x16000000,
    #[value(name = "recorder-max-channels")]
    RecorderMaxChannels = 0x16000100,
    #[value(name = "recorder-max-points")]
    RecorderMaxPoints = 0x16000200,
    #[value(name = "recorder-chan-num")]
    RecorderChanNumber = 0x16000300,
    #[value(name = "drc-data-source")]
    DRCDataSource = 0x16000700,
    #[value(name = "drc-record-option")]
    DRCRecordOption = 0x16000701,

    // ==================== CTO/Trigger (0x18) ====================
    #[value(name = "cto-trigger-step")]
    CTOTriggerStep = 0x18000201,
    #[value(name = "cto-axis")]
    CTOAxis = 0x18000202,
    #[value(name = "cto-trigger-mode")]
    CTOTriggerMode = 0x18000203,
    #[value(name = "cto-min-threshold")]
    CTOMinThreshold = 0x18000205,
    #[value(name = "cto-max-threshold")]
    CTOMaxThreshold = 0x18000206,
    #[value(name = "cto-polarity")]
    CTOPolarity = 0x18000207,
    #[value(name = "cto-start-threshold")]
    CTOStartThreshold = 0x18000208,
    #[value(name = "cto-stop-threshold")]
    CTOStopThreshold = 0x18000209,

    // ==================== Firmware Update (0xFFFF) ====================
    #[value(name = "fw-mark")]
    FirmwareMark = 0xFFFF0001,
    #[value(name = "fw-crc")]
    FirmwareCRC = 0xFFFF0002,
    #[value(name = "fw-desc-crc")]
    FirmwareDescCRC = 0xFFFF0003,
    #[value(name = "fw-desc-version")]
    FirmwareDescVersion = 0xFFFF0004,
    #[value(name = "fw-matchcode")]
    FirmwareMatchcode = 0xFFFF0006,
    #[value(name = "hw-matchcode")]
    HardwareMatchcode = 0xFFFF0007,
    #[value(name = "fw-version")]
    FirmwareVersion = 0xFFFF0008,
    #[value(name = "fw-max-size")]
    FirmwareMaxSize = 0xFFFF000B,
    #[value(name = "fw-device")]
    FirmwareDevice = 0xFFFF000C,
    #[value(name = "fw-short-desc")]
    FirmwareShortDesc = 0xFFFF000D,
    #[value(name = "fw-date")]
    FirmwareDate = 0xFFFF000E,
    #[value(name = "fw-developer")]
    FirmwareDeveloper = 0xFFFF000F,
    #[value(name = "fw-length")]
    FirmwareLength = 0xFFFF0010,
    #[value(name = "fw-compat")]
    FirmwareCompatibility = 0xFFFF0011,
    #[value(name = "fw-rel-addr")]
    FirmwareRelAddress = 0xFFFF0012,
    #[value(name = "fw-device-type")]
    FirmwareDeviceType = 0xFFFF0013,
    #[value(name = "hw-revision")]
    HardwareRevision = 0xFFFF0014,
    #[value(name = "fw-dest-addr")]
    FirmwareDestAddr = 0xFFFF0015,
    #[value(name = "fw-config")]
    FirmwareConfiguration = 0xFFFF0016,
}

impl SpaParam {
    /// Get the hex address for this parameter.
    pub fn address(self) -> u32 {
        self as u32
    }

    /// Get the MaxItem value for this parameter (from HPA? output).
    ///
    /// This determines how to query the parameter:
    /// - 1 = System-wide, query without axis: `SPA? 0xADDR`
    /// - 4 = Per-axis, query for each axis 1-4: `SPA? <axis> 0xADDR`
    /// - Other values indicate per-channel/per-table parameters
    ///
    /// Based on empirical analysis of HPA? output from E-727.
    pub fn max_items(self) -> u8 {
        let addr = self.address();
        let high_byte = (addr >> 24) as u8;

        match high_byte {
            // System-wide parameters (MaxItem=1)
            0x0D => 1, // Device info (S/N, hardware name)
            0x0E => 1, // System config (timing, licenses, channel counts)
            0x11 => 1, // Network/UART config

            // Per-axis parameters (MaxItem=4)
            0x02 => 7, // Sensor interface (varies, using 7 for input channels)
            0x03 => 2, // Input/sensor corrections (2 for most)
            0x05 => 1, // Digital filter (system-wide)
            0x06 => 1, // Analog target (system-wide)
            0x07 => 4, // Servo loop parameters
            0x08 => 4, // Notch filters
            0x09 => 4, // Piezo driving factors
            0x0A => 4, // Output selection
            0x0B => 4, // Output/amplifier
            0x0C => 4, // Soft voltage limits
            0x10 => 4, // FastIF

            // Special cases
            0x0F => 3, // ID-Chip/Stage (3 channels)
            0x13 => 1, // Wave generator (mostly system-wide)
            0x14 => 4, // DDL (per-axis)
            0x16 => 1, // Data recorder config (system-wide, but DRC tables are per-table)
            0x18 => 3, // CTO triggers (3 trigger outputs)
            0xFF => 1, // Firmware update (system-wide)

            _ => 4, // Default to per-axis
        }
    }

    /// Returns true if this parameter is system-wide (no axis needed).
    pub fn is_system_wide(self) -> bool {
        self.max_items() == 1
    }
}
