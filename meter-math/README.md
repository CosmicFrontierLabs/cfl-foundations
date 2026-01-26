# meter-math

Mathematical algorithms for astronomical simulations and star tracking systems.

## Overview

This crate provides core mathematical primitives used across the meter-sim project for celestial mechanics, point cloud alignment, and signal processing.

## Modules

| Module | Description |
|--------|-------------|
| `quaternion` | 3D rotation representation with axis-angle construction and vector rotation |
| `icp` | Iterative Closest Point algorithm for aligning detected stars to catalog positions |
| `spline` | Cubic spline interpolation for smooth trajectory generation |
| `bilinear` | Bilinear interpolation for 2D grid sampling |
| `matrix2` | 2D transformation matrices (rotation, scaling, inversion) |
| `stats` | Statistical functions (median, Pearson correlation, KS test for normality) |

## Usage

```rust
use meter_math::{Quaternion, iterative_closest_point};
use nalgebra::Vector3;

// Create a rotation quaternion (45 degrees around z-axis)
let axis = Vector3::new(0.0, 0.0, 1.0);
let angle = std::f64::consts::FRAC_PI_4;
let q = Quaternion::from_axis_angle(&axis, angle);

// Rotate a vector
let v = Vector3::new(1.0, 0.0, 0.0);
let rotated = q.rotate_vector(&v);
```

## Documentation

Generate and view the full API documentation:

```bash
cargo doc --package meter-math --open
```
