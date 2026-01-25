//! Hardware drivers for test bench equipment.
//!
//! This crate provides drivers for various hardware components used in the
//! optical test bench. Each driver is feature-gated for optional compilation.
//!
//! # Features
//!
//! ## Individual Drivers
//! - `nsv455` - NSV455 V4L2 camera driver (Linux only)
//! - `orin` - Jetson Orin GPIO and system monitoring (Linux only)
//! - `pi-fsm` - PI E-727 FSM controller (Ethernet, cross-platform)
//! - `exail` - Exail gyroscope serial protocol
//! - `ftdi` - FTDI serial adapter utilities
//! - `exolambda` - ExoLambda packet protocol
//! - `playerone` - PlayerOne astronomy camera SDK
//!
//! ## Convenience Features
//! - `full-linux` - All drivers for Linux deployments
//! - `cross-platform` - Drivers that work on any OS (pi-fsm, exail, ftdi, exolambda)
//! - `ci-testable` - Drivers with unit tests runnable in CI (no hardware needed)

#[cfg(feature = "exail")]
pub mod exail;

#[cfg(feature = "exolambda")]
pub mod exolambda;

#[cfg(feature = "ftdi")]
pub mod ftdi;

#[cfg(all(target_os = "linux", feature = "nsv455"))]
pub mod nsv455;

#[cfg(all(target_os = "linux", feature = "orin"))]
pub mod orin;

#[cfg(feature = "pi-fsm")]
pub mod pi;

#[cfg(feature = "playerone")]
pub mod poa;

#[cfg(feature = "playerone")]
pub use playerone_sdk;
