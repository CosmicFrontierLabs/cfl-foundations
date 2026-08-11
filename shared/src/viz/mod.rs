//! Visualization toolkit for astronomical data analysis.

use std::fmt;
use thiserror::Error;

/// Comprehensive error types for visualization operations.
///
/// Provides detailed error reporting for all visualization failures including
/// data validation errors, formatting issues, and configuration problems.
/// Each error type includes specific context to help with debugging and
/// user feedback.
#[derive(Debug, Error)]
pub enum VizError {
    /// Histogram creation or analysis error.
    ///
    /// Includes issues with bin configuration, data validation,
    /// and statistical computation failures.
    #[error("Histogram error: {0}")]
    HistogramError(String),

    /// Text formatting or I/O error.
    ///
    /// Includes failures during ASCII output generation,
    /// string formatting, and file I/O operations.
    #[error("Formatting error: {0}")]
    FmtError(#[from] fmt::Error),
}

/// Standard Result type for all visualization operations.
///
/// Provides consistent error handling across the entire visualization
/// toolkit with detailed error context for debugging and user feedback.
pub type Result<T> = std::result::Result<T, VizError>;

pub mod density_map;
pub mod histogram;
