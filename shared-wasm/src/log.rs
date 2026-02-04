//! Log streaming types.

use serde::{Deserialize, Serialize};

/// Log level for streamed log entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    /// Numeric severity rank (higher = more severe).
    pub fn rank(&self) -> u8 {
        match self {
            LogLevel::Trace => 0,
            LogLevel::Debug => 1,
            LogLevel::Info => 2,
            LogLevel::Warn => 3,
            LogLevel::Error => 4,
        }
    }

    /// Check if this level passes a minimum filter.
    pub fn passes_filter(&self, min_level: &LogLevel) -> bool {
        self.rank() >= min_level.rank()
    }

    /// Lowercase string for URL query parameters.
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "trace",
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
        }
    }

    /// Get CSS color for this log level.
    pub fn color(&self) -> &'static str {
        match self {
            LogLevel::Trace => "#888888",
            LogLevel::Debug => "#00aaaa",
            LogLevel::Info => "#00ff00",
            LogLevel::Warn => "#ffaa00",
            LogLevel::Error => "#ff4444",
        }
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// A log entry for WebSocket streaming.
///
/// Serialized as JSON for transmission over WebSocket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Unix timestamp in milliseconds
    pub timestamp_ms: u64,
    /// Log level
    pub level: LogLevel,
    /// Target (module path)
    pub target: String,
    /// Log message
    pub message: String,
}
