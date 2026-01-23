//! HTTP client for interacting with calibrate_serve API.
//!
//! This module provides a unified client that works in both native Rust
//! and WASM (frontend) environments. All API interactions are consolidated
//! here for consistent error handling and type safety.

use gloo_net::http::Request;
use serde::{de::DeserializeOwned, Serialize};

use crate::{DisplayInfo, PatternCommand, PatternConfigResponse, SchemaResponse};

/// Error type for calibrate server operations.
#[derive(Debug, thiserror::Error)]
pub enum CalibrateError {
    /// HTTP request failed
    #[error("HTTP error: {0}")]
    Http(String),
    /// Failed to parse response
    #[error("Parse error: {0}")]
    Parse(String),
    /// Connection failed
    #[error("Connection error: {0}")]
    Connection(String),
    /// Request timed out
    #[error("Timeout")]
    Timeout,
    /// Server returned an error status
    #[error("Server error (status {status}): {message}")]
    ServerError { status: u16, message: String },
}

impl From<gloo_net::Error> for CalibrateError {
    fn from(err: gloo_net::Error) -> Self {
        CalibrateError::Http(err.to_string())
    }
}

/// Client for interacting with calibrate_serve HTTP API.
///
/// Works in both native Rust and WASM environments.
#[derive(Debug, Clone)]
pub struct CalibrateServerClient {
    base_url: String,
}

impl CalibrateServerClient {
    /// Create a new client pointing to the given base URL.
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL of the calibrate_serve (e.g., "http://localhost:3001")
    pub fn new(base_url: &str) -> Self {
        // Remove trailing slash if present
        let base_url = base_url.trim_end_matches('/').to_string();
        Self { base_url }
    }

    /// Create a client for same-origin web requests.
    ///
    /// Uses relative URLs (empty base) which works in WASM when the frontend
    /// is served from the same origin as the API. Panics if called outside WASM.
    pub fn for_web() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            Self::new("")
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            unreachable!("for_web() is only available in WASM builds")
        }
    }

    /// Get the base URL this client is configured for.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    // === Internal HTTP helpers ===

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, CalibrateError> {
        let url = format!("{}{}", self.base_url, path);
        let response = Request::get(&url).send().await?;

        if !response.ok() {
            return Err(CalibrateError::ServerError {
                status: response.status(),
                message: response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string()),
            });
        }

        response
            .json::<T>()
            .await
            .map_err(|e| CalibrateError::Parse(e.to_string()))
    }

    #[allow(dead_code)]
    async fn post<T: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R, CalibrateError> {
        let url = format!("{}{}", self.base_url, path);
        let response = Request::post(&url)
            .json(body)
            .map_err(|e| CalibrateError::Parse(e.to_string()))?
            .send()
            .await?;

        if !response.ok() {
            return Err(CalibrateError::ServerError {
                status: response.status(),
                message: response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string()),
            });
        }

        response
            .json::<R>()
            .await
            .map_err(|e| CalibrateError::Parse(e.to_string()))
    }

    async fn post_no_response<T: Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<(), CalibrateError> {
        let url = format!("{}{}", self.base_url, path);
        let response = Request::post(&url)
            .json(body)
            .map_err(|e| CalibrateError::Parse(e.to_string()))?
            .send()
            .await?;

        if !response.ok() {
            return Err(CalibrateError::ServerError {
                status: response.status(),
                message: response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string()),
            });
        }

        Ok(())
    }

    // === Display Info ===

    /// Get display information.
    ///
    /// # Returns
    ///
    /// Display information including dimensions, pixel pitch, and name.
    pub async fn get_display_info(&self) -> Result<DisplayInfo, CalibrateError> {
        self.get("/info").await
    }

    // === Schema ===

    /// Get pattern schema.
    ///
    /// # Returns
    ///
    /// Schema describing available patterns and their controls.
    pub async fn get_schema(&self) -> Result<SchemaResponse, CalibrateError> {
        self.get("/schema").await
    }

    // === Pattern Configuration ===

    /// Get current pattern configuration.
    ///
    /// # Returns
    ///
    /// The current pattern ID, parameter values, and invert state.
    pub async fn get_config(&self) -> Result<PatternConfigResponse, CalibrateError> {
        self.get("/config").await
    }

    /// Update pattern configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The pattern configuration to apply
    pub async fn set_config(&self, config: &PatternConfigRequest) -> Result<(), CalibrateError> {
        self.post_no_response("/config", config).await
    }

    // === Pattern Commands (RemoteControlled mode) ===

    /// Get current pattern command.
    ///
    /// # Returns
    ///
    /// The current pattern command (Spot, SpotGrid, Uniform, or Clear).
    pub async fn get_pattern(&self) -> Result<PatternCommand, CalibrateError> {
        self.get("/pattern").await
    }

    /// Send pattern command.
    ///
    /// # Arguments
    ///
    /// * `command` - The pattern command to execute
    ///
    /// Note: The display must be in RemoteControlled mode for these
    /// commands to have visible effect.
    pub async fn set_pattern(&self, command: &PatternCommand) -> Result<(), CalibrateError> {
        self.post_no_response("/pattern", command).await
    }

    // === Image URLs ===
    // These return URLs rather than fetching data, since images are typically
    // loaded directly by the browser/frontend.

    /// Get the URL for JPEG pattern endpoint.
    pub fn jpeg_url(&self) -> String {
        format!("{}/jpeg", self.base_url)
    }
}

/// Request body for updating pattern configuration.
#[derive(Debug, Clone, Serialize)]
pub struct PatternConfigRequest {
    /// Pattern type identifier
    pub pattern_id: String,
    /// Pattern-specific parameter values
    pub values: serde_json::Map<String, serde_json::Value>,
    /// Whether to invert colors
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invert: Option<bool>,
    /// Enable gyro emission (if FTDI configured)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emit_gyro: Option<bool>,
    /// Plate scale in arcsec/pixel for gyro emission
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plate_scale: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_url_construction() {
        let client = CalibrateServerClient::new("http://localhost:3001");
        assert_eq!(client.base_url(), "http://localhost:3001");
        assert_eq!(client.jpeg_url(), "http://localhost:3001/jpeg");
    }

    #[test]
    fn test_client_strips_trailing_slash() {
        let client = CalibrateServerClient::new("http://localhost:3001/");
        assert_eq!(client.base_url(), "http://localhost:3001");
    }
}
