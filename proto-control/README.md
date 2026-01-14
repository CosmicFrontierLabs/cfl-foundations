# Proto-Control

Line-of-Sight Control Interface for the meter-sim spacecraft pointing control system.

## Overview

This crate defines the protocol and data structures for the line-of-sight (LOS)
control algorithm to interface with the spacecraft simulation system. The
`StateEstimator` trait provides the interface for state estimation and control logic.

## Architecture

### Control Loop Timing

- **`GyroTick`** - 500Hz tick counter synchronized with gyroscope measurements
- **`Timestamp`** - Microseconds (u64) for sensor timing precision

### Sensor Inputs

| Struct | Description | Rate |
|--------|-------------|------|
| `GyroReadout` | 3-axis integrated angles (radians) from Exail gyroscope | Every gyro tick (500Hz) |
| `FgsReadout` | Fine Guidance System 2D angular position with variance (arcseconds) | Lower rate (camera-based) |
| `FsmReadout` | Fast Steering Mirror voltage feedback (vx/vy volts) | Every gyro tick |

### Control Output

- **`FsmCommand`** - Voltage commands to drive the Fast Steering Mirror (vx/vy volts)

### State Container

- **`EstimatorState`** - Packages one complete control cycle: gyro tick, all sensor
  readings, and the computed FSM command output

## The StateEstimator Trait

This is the interface for the LOS control algorithm:

```text
f: (state_history, gyro, fsm_readout, fgs_readout?) → EstimatorState
```

Key design points:

- Receives FIFO history of previous outputs (oldest → newest)
- FGS readout is optional (camera rate slower than 500Hz control loop)
- Caller handles FSM command execution timing and LOS updates to payload computer

## System Context

The system implements classic sensor fusion for spacecraft fine pointing:

1. **High-rate gyro measurement** - Exail gyroscope provides integrated angles at 500Hz
2. **Periodic FGS corrections** - Camera-based Fine Guidance System provides absolute reference
3. **FSM actuation** - Fast Steering Mirror commands for fine pointing stabilization

```text
+-------------+     +-----------------+     +-------------+
| Exail Gyro  |---->|                 |---->|     FSM     |
|   (500Hz)   |     |  StateEstimator |     |  (actuator) |
+-------------+     |                 |     +-------------+
                    |  (LOS control   |
+-------------+     |   algorithm)    |     +-------------+
|     FGS     |---->|                 |---->|   Payload   |
|  (camera)   |     +-----------------+     |  Computer   |
+-------------+           ^                 | (LOS update)|
                          |                 +-------------+
                    +-----+-----+
                    |   state   |
                    |  history  |
                    +-----------+
```
