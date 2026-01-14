# FSM Axis Calibration Specification

## Overview

This document specifies the calibration procedure for mapping PI S-330 Fast Steering Mirror (FSM) axes to FGS sensor coordinates. The calibration determines how FSM commands translate to centroid motion on the detector.

## Problem Statement

The PI S-330 has two tilt axes (Axis 1, Axis 2) that deflect the optical beam. The relationship between FSM commands (in microradians) and resulting centroid motion (in pixels) depends on:

- Physical mounting orientation of the S-330
- Position in optical path (before/after objective lens)
- Fold mirrors in optical train
- Detector orientation and pixel pitch
- Reflection geometry (2x beam deflection factor)

Rather than computing this from first principles (error-prone), we measure it empirically.

## Calibration Procedure

### High-Level Flow

```
┌─────────────────────────────────────────────────────────────┐
│  1. Acquire guide star → stable centroid baseline           │
│  2. Wiggle FSM Axis 1 (sinusoid) → record centroid trace    │
│  3. Wiggle FSM Axis 2 (sinusoid) → record centroid trace    │
│  4. Fit sinusoids to extract response vectors               │
│  5. Build 2×2 transform matrix                              │
│  6. Verify with circular motion test                        │
└─────────────────────────────────────────────────────────────┘
```

### Sinusoidal Wiggle Method

Apply a sinusoidal command to one FSM axis while holding the other at zero:

```
FSM_axis1(t) = A * sin(2π * f * t)
FSM_axis2(t) = 0
```

Where:
- `A` = wiggle amplitude (suggested: 50-200 µrad)
- `f` = wiggle frequency (suggested: 0.5-2 Hz)
- Duration: 5-10 complete cycles

Record centroid position `(cx, cy)` at each camera frame. The centroid will trace an ellipse (or line) in sensor space.

### Response Vector Extraction

Fit the centroid traces to sinusoids at the excitation frequency:

```
cx(t) ≈ cx0 + Rx1 * sin(2π * f * t + φx1)
cy(t) ≈ cy0 + Ry1 * sin(2π * f * t + φy1)
```

The response vector for Axis 1 is:
```
v1 = (Rx1 / A, Ry1 / A)   [pixels per µrad]
```

Repeat for Axis 2 to get `v2`.

### Transform Matrix Construction

The calibration produces a 2×2 matrix mapping FSM commands to sensor motion:

```
┌          ┐   ┌              ┐   ┌        ┐
│ Δcx      │   │ v1x    v2x   │   │ axis1  │
│          │ = │              │ × │        │
│ Δcy      │   │ v1y    v2y   │   │ axis2  │
└          ┘   └              ┘   └        ┘
```

For closed-loop control, we need the inverse:

```
┌        ┐   ┌              ┐   ┌          ┐
│ axis1  │   │ inv[0][0]    │   │ Δcx      │
│        │ = │   inv[0][1]  │ × │          │
│ axis2  │   │ inv[1][0]    │   │ Δcy      │
└        ┘   │   inv[1][1]  │   └          ┘
              └              ┘
```

### Verification

After calibration, command a circle in FSM space:

```
axis1(t) = R * cos(2π * f * t)
axis2(t) = R * sin(2π * f * t)
```

The centroid should trace a corresponding ellipse/circle in sensor space. Measure:
- Shape match (is it circular or elliptical?)
- Orientation match (does predicted orientation match observed?)
- RMS error between predicted and measured positions

## Data Structures

### CalibrationConfig

```rust
pub struct FsmCalibrationConfig {
    /// Wiggle amplitude in microradians
    pub wiggle_amplitude_urad: f64,
    /// Wiggle frequency in Hz
    pub wiggle_frequency_hz: f64,
    /// Number of complete cycles per axis
    pub wiggle_cycles: usize,
    /// Verification circle radius in microradians
    pub verify_radius_urad: f64,
    /// Minimum acceptable R² for sinusoid fit
    pub min_fit_r_squared: f64,
}

impl Default for FsmCalibrationConfig {
    fn default() -> Self {
        Self {
            wiggle_amplitude_urad: 100.0,
            wiggle_frequency_hz: 1.0,
            wiggle_cycles: 5,
            verify_radius_urad: 150.0,
            min_fit_r_squared: 0.95,
        }
    }
}
```

### CalibrationResult

```rust
pub struct FsmAxisCalibration {
    /// Transform from FSM µrad to sensor pixels: [pixels/µrad]
    /// sensor_delta = fsm_to_sensor * fsm_command
    pub fsm_to_sensor: [[f64; 2]; 2],

    /// Transform from sensor pixels to FSM µrad: [µrad/pixel]
    /// fsm_command = sensor_to_fsm * sensor_delta
    pub sensor_to_fsm: [[f64; 2]; 2],

    /// Fit quality metrics
    pub axis1_r_squared: f64,
    pub axis2_r_squared: f64,

    /// Verification metrics
    pub verification_rms_error_pixels: f64,
    pub verification_max_error_pixels: f64,

    /// Timestamp of calibration
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Configuration used for this calibration
    pub config: FsmCalibrationConfig,
}
```

### CalibrationTrace

```rust
pub struct CalibrationTrace {
    /// Timestamps relative to start (seconds)
    pub time_s: Vec<f64>,
    /// FSM Axis 1 commands (µrad)
    pub fsm_axis1: Vec<f64>,
    /// FSM Axis 2 commands (µrad)
    pub fsm_axis2: Vec<f64>,
    /// Measured centroid X (pixels)
    pub centroid_x: Vec<f64>,
    /// Measured centroid Y (pixels)
    pub centroid_y: Vec<f64>,
    /// Frame indices for correlation
    pub frame_indices: Vec<u64>,
}
```

## Subcomponents

The calibration system breaks down into the following testable components:

### 1. Sinusoid Generator (`fsm_calibration::generator`)

**Purpose**: Generate smooth sinusoidal command sequences for FSM

**Interface**:
```rust
pub struct SinusoidGenerator {
    amplitude: f64,
    frequency: f64,
    sample_rate: f64,
}

impl SinusoidGenerator {
    pub fn new(amplitude: f64, frequency: f64, sample_rate: f64) -> Self;
    pub fn generate(&self, duration_s: f64) -> Vec<f64>;
    pub fn sample_at(&self, time_s: f64) -> f64;
}
```

**Tests**:
- Verify amplitude is correct at peaks
- Verify frequency matches expected zero crossings
- Verify continuity (no discontinuities)

### 2. Sinusoid Fitter (`fsm_calibration::fitter`)

**Purpose**: Extract amplitude and phase from noisy centroid data

**Interface**:
```rust
pub struct SinusoidFit {
    pub amplitude: f64,
    pub phase: f64,
    pub offset: f64,
    pub r_squared: f64,
}

pub fn fit_sinusoid(
    data: &[f64],
    time_s: &[f64],
    frequency: f64,
) -> Result<SinusoidFit, FitError>;
```

**Tests**:
- Fit perfect sinusoid → exact recovery
- Fit noisy sinusoid → amplitude within tolerance
- Fit with DC offset → correctly extracted
- Non-sinusoidal data → low R², error returned
- Phase recovery accuracy

### 3. Matrix Builder (`fsm_calibration::matrix`)

**Purpose**: Construct and validate transform matrices

**Interface**:
```rust
pub fn build_transform_matrix(
    axis1_response: (f64, f64),  // (dx, dy) per µrad
    axis2_response: (f64, f64),
) -> Result<[[f64; 2]; 2], MatrixError>;

pub fn invert_2x2(matrix: [[f64; 2]; 2]) -> Result<[[f64; 2]; 2], MatrixError>;

pub fn apply_transform(matrix: &[[f64; 2]; 2], input: (f64, f64)) -> (f64, f64);
```

**Tests**:
- Identity matrix → no change
- Rotation matrix → correct rotation
- Inversion → M * M^-1 = I
- Singular matrix → appropriate error
- Known transforms → correct output

### 4. Calibration Executor (`fsm_calibration::executor`)

**Purpose**: Orchestrate the full calibration sequence

**Interface**:
```rust
pub struct CalibrationExecutor<F: FsmInterface, C: CameraInterface> {
    fsm: F,
    camera: C,
    fgs: Monocle,
    config: FsmCalibrationConfig,
}

impl<F, C> CalibrationExecutor<F, C> {
    pub async fn run_calibration(&mut self) -> Result<FsmAxisCalibration, CalibError>;
    pub async fn run_verification(&mut self, calib: &FsmAxisCalibration) -> Result<VerificationResult, CalibError>;
}
```

**Tests** (with mock FSM/camera):
- Simulated perfect response → correct matrix
- Simulated rotated axes → correct matrix
- Simulated inverted axis → correct sign
- Timeout handling
- SNR dropout during calibration → graceful error

### 5. Calibration Verifier (`fsm_calibration::verifier`)

**Purpose**: Validate calibration with circular motion test

**Interface**:
```rust
pub struct VerificationResult {
    pub commanded_positions: Vec<(f64, f64)>,
    pub predicted_centroids: Vec<(f64, f64)>,
    pub measured_centroids: Vec<(f64, f64)>,
    pub errors: Vec<f64>,
    pub rms_error: f64,
    pub max_error: f64,
    pub passed: bool,
}

pub fn verify_calibration(
    traces: &CalibrationTrace,
    calibration: &FsmAxisCalibration,
    threshold_pixels: f64,
) -> VerificationResult;
```

**Tests**:
- Perfect calibration → zero error
- Scaled calibration (wrong gain) → proportional error
- Rotated calibration (wrong angle) → systematic error pattern

### 6. Persistence Layer (`fsm_calibration::storage`)

**Purpose**: Save/load calibration data

**Interface**:
```rust
pub fn save_calibration(
    calib: &FsmAxisCalibration,
    path: &Path,
) -> Result<(), IoError>;

pub fn load_calibration(path: &Path) -> Result<FsmAxisCalibration, IoError>;

pub fn save_trace(
    trace: &CalibrationTrace,
    path: &Path,
) -> Result<(), IoError>;
```

**Tests**:
- Round-trip save/load preserves all fields
- Invalid file → appropriate error
- Version compatibility (if format changes)

## Integration Points

### With FGS Server

The calibration can be triggered from the FGS web interface:

```
GET  /api/fsm/calibration/status     → current calibration state
POST /api/fsm/calibration/start      → begin calibration
GET  /api/fsm/calibration/progress   → SSE stream of progress
POST /api/fsm/calibration/abort      → cancel in-progress calibration
GET  /api/fsm/calibration/result     → get last calibration result
POST /api/fsm/calibration/verify     → run verification sequence
```

### With LOS Controller

The calibration matrix integrates with the existing LOS controller:

```rust
impl LosController {
    pub fn set_axis_calibration(&mut self, calib: FsmAxisCalibration);

    pub fn update(&mut self, centroid_error: (f64, f64)) -> FsmCommand {
        // Compute correction in sensor-aligned frame
        let (corr_x, corr_y) = self.compute_correction(centroid_error);

        // Transform to FSM physical axes using calibration
        let axis1 = self.calib.sensor_to_fsm[0][0] * corr_x
                  + self.calib.sensor_to_fsm[0][1] * corr_y;
        let axis2 = self.calib.sensor_to_fsm[1][0] * corr_x
                  + self.calib.sensor_to_fsm[1][1] * corr_y;

        FsmCommand { axis1, axis2 }
    }
}
```

## Error Handling

### Calibration Failures

| Error | Cause | Recovery |
|-------|-------|----------|
| `NoGuideStar` | Cannot acquire stable centroid | Adjust exposure, check focus |
| `LowFitQuality` | R² below threshold | Increase amplitude, reduce noise |
| `SingularMatrix` | Axes are parallel/degenerate | Check FSM connections, hardware |
| `VerificationFailed` | Measured != predicted | Re-run calibration |
| `FsmTimeout` | FSM not responding | Check network, power |
| `SnrDropout` | Lost guide star during cal | Increase brightness, reduce motion |

### Graceful Degradation

If calibration fails or is unavailable:
1. Log warning
2. Use identity matrix (assume direct X→X, Y→Y mapping)
3. Display warning in UI
4. Allow manual override of calibration values

## Implementation Order

1. **Phase 1: Core Math** (no hardware)
   - Sinusoid generator
   - Sinusoid fitter
   - Matrix operations
   - Unit tests with synthetic data

2. **Phase 2: Mock Integration**
   - Mock FSM interface
   - Mock camera/FGS
   - Full executor tests with simulated responses
   - Verify edge cases

3. **Phase 3: Hardware Integration**
   - Connect to real S-330
   - Connect to real camera
   - Manual testing with hardware
   - Calibration binary for standalone use

4. **Phase 4: Server Integration**
   - REST API endpoints
   - Frontend UI for calibration
   - Progress streaming
   - Calibration persistence

## File Organization

```
monocle/
├── src/
│   ├── fsm_calibration/
│   │   ├── mod.rs
│   │   ├── config.rs         # CalibrationConfig
│   │   ├── generator.rs      # Sinusoid generator
│   │   ├── fitter.rs         # Sinusoid fitting
│   │   ├── matrix.rs         # Matrix operations
│   │   ├── executor.rs       # Calibration orchestration
│   │   ├── verifier.rs       # Verification logic
│   │   └── storage.rs        # Persistence
│   └── controllers/
│       └── los_controller.rs # Updated with calibration support
├── tests/
│   └── fsm_calibration_tests.rs
```

## References

- PI S-330 datasheet (axis definitions, range specifications)
- PI E-727 GCS command reference
- Existing `hardware/src/pi/` driver code
- `monocle/src/controllers/los_controller.rs`
