//! HTTP client for interacting with fgs_server API.
//!
//! This module provides a unified client that works in both native Rust
//! and WASM (frontend) environments. All API interactions are consolidated
//! here for consistent error handling and type safety.

use gloo_net::http::Request;
use serde::{de::DeserializeOwned, Serialize};

use crate::{
    CameraStats, ExportSettings, ExportStatus, FsmMoveRequest, FsmStatus, RawFrameResponse,
    StarDetectionSettings, TrackingEnableRequest, TrackingSettings, TrackingState, TrackingStatus,
};

/// Error type for FGS server operations.
#[derive(Debug, thiserror::Error)]
pub enum FgsError {
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

impl From<gloo_net::Error> for FgsError {
    fn from(err: gloo_net::Error) -> Self {
        FgsError::Http(err.to_string())
    }
}

/// Client for interacting with fgs_server HTTP API.
///
/// Works in both native Rust and WASM environments.
#[derive(Debug, Clone)]
pub struct FgsServerClient {
    base_url: String,
}

impl FgsServerClient {
    /// Create a new client pointing to the given base URL.
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL of the fgs_server (e.g., "http://localhost:3000")
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

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, FgsError> {
        let url = format!("{}{}", self.base_url, path);
        let response = Request::get(&url).send().await?;

        if !response.ok() {
            return Err(FgsError::ServerError {
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
            .map_err(|e| FgsError::Parse(e.to_string()))
    }

    async fn post<T: Serialize, R: DeserializeOwned>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<R, FgsError> {
        let url = format!("{}{}", self.base_url, path);
        let response = Request::post(&url)
            .json(body)
            .map_err(|e| FgsError::Parse(e.to_string()))?
            .send()
            .await?;

        if !response.ok() {
            return Err(FgsError::ServerError {
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
            .map_err(|e| FgsError::Parse(e.to_string()))
    }

    async fn post_no_response<T: Serialize>(&self, path: &str, body: &T) -> Result<(), FgsError> {
        let url = format!("{}{}", self.base_url, path);
        let response = Request::post(&url)
            .json(body)
            .map_err(|e| FgsError::Parse(e.to_string()))?
            .send()
            .await?;

        if !response.ok() {
            return Err(FgsError::ServerError {
                status: response.status(),
                message: response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string()),
            });
        }

        Ok(())
    }

    // === Tracking Control ===

    /// Get current tracking status.
    ///
    /// # Returns
    ///
    /// The current tracking status including state, position, and statistics.
    pub async fn get_tracking_status(&self) -> Result<TrackingStatus, FgsError> {
        self.get("/tracking/status").await
    }

    /// Enable or disable tracking.
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to enable or disable tracking
    ///
    /// # Returns
    ///
    /// The updated tracking status after the change.
    pub async fn set_tracking_enabled(&self, enabled: bool) -> Result<TrackingStatus, FgsError> {
        self.post("/tracking/enable", &TrackingEnableRequest { enabled })
            .await
    }

    /// Get tracking settings.
    ///
    /// # Returns
    ///
    /// The current tracking algorithm settings.
    pub async fn get_tracking_settings(&self) -> Result<TrackingSettings, FgsError> {
        self.get("/tracking/settings").await
    }

    /// Update tracking settings.
    ///
    /// # Arguments
    ///
    /// * `settings` - The new tracking settings to apply
    pub async fn set_tracking_settings(&self, settings: &TrackingSettings) -> Result<(), FgsError> {
        self.post_no_response("/tracking/settings", settings).await
    }

    /// Check if tracking is currently active (has lock).
    ///
    /// # Returns
    ///
    /// `true` if the system is in the Tracking state, `false` otherwise.
    pub async fn is_tracking_active(&self) -> Result<bool, FgsError> {
        let status = self.get_tracking_status().await?;
        Ok(matches!(status.state, TrackingState::Tracking { .. }))
    }

    // === Export Control ===

    /// Get export status.
    ///
    /// # Returns
    ///
    /// The current export status including settings and statistics.
    pub async fn get_export_status(&self) -> Result<ExportStatus, FgsError> {
        self.get("/tracking/export").await
    }

    /// Update export settings.
    ///
    /// # Arguments
    ///
    /// * `settings` - The new export settings to apply
    pub async fn set_export_settings(&self, settings: &ExportSettings) -> Result<(), FgsError> {
        self.post_no_response("/tracking/export", settings).await
    }

    // === Camera Control ===

    /// Get camera statistics.
    ///
    /// # Returns
    ///
    /// Camera statistics including frame count, FPS, temperatures, and histogram.
    pub async fn get_camera_stats(&self) -> Result<CameraStats, FgsError> {
        self.get("/stats").await
    }

    /// Get raw frame data.
    ///
    /// # Returns
    ///
    /// Raw frame response including base64-encoded image data.
    pub async fn get_raw_frame(&self) -> Result<RawFrameResponse, FgsError> {
        self.get("/raw").await
    }

    // === FSM Control ===

    /// Get FSM (Fast Steering Mirror) status.
    ///
    /// # Returns
    ///
    /// The current FSM status including position and limits.
    pub async fn get_fsm_status(&self) -> Result<FsmStatus, FgsError> {
        self.get("/fsm/status").await
    }

    /// Move FSM to a position.
    ///
    /// # Arguments
    ///
    /// * `x_urad` - Target X position in microradians
    /// * `y_urad` - Target Y position in microradians
    pub async fn move_fsm(&self, x_urad: f64, y_urad: f64) -> Result<(), FgsError> {
        self.post_no_response("/fsm/move", &FsmMoveRequest { x_urad, y_urad })
            .await
    }

    // === Star Detection Control ===

    /// Get star detection settings.
    ///
    /// # Returns
    ///
    /// The current star detection algorithm settings.
    pub async fn get_star_detection_settings(&self) -> Result<StarDetectionSettings, FgsError> {
        self.get("/detection/settings").await
    }

    /// Update star detection settings.
    ///
    /// # Arguments
    ///
    /// * `settings` - The new star detection settings to apply
    ///
    /// # Returns
    ///
    /// The updated settings after applying changes.
    pub async fn set_star_detection_settings(
        &self,
        settings: &StarDetectionSettings,
    ) -> Result<StarDetectionSettings, FgsError> {
        self.post("/detection/settings", settings).await
    }

    // === Image URLs ===
    // These return URLs rather than fetching data, since images are typically
    // loaded directly by the browser/frontend.

    /// Get the URL for JPEG frame endpoint.
    pub fn jpeg_url(&self) -> String {
        format!("{}/jpeg", self.base_url)
    }

    /// Get the URL for annotated frame endpoint.
    pub fn annotated_url(&self) -> String {
        format!("{}/annotated", self.base_url)
    }

    /// Get the URL for zoom-annotated frame endpoint.
    pub fn zoom_annotated_url(&self) -> String {
        format!("{}/zoom-annotated", self.base_url)
    }

    /// Get the URL for SVG overlay endpoint.
    pub fn overlay_svg_url(&self) -> String {
        format!("{}/overlay-svg", self.base_url)
    }

    /// Get the URL for tracking events SSE endpoint.
    pub fn tracking_events_url(&self) -> String {
        format!("{}/tracking/events", self.base_url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_url_construction() {
        let client = FgsServerClient::new("http://localhost:3000");
        assert_eq!(client.base_url(), "http://localhost:3000");
        assert_eq!(client.jpeg_url(), "http://localhost:3000/jpeg");
        assert_eq!(
            client.tracking_events_url(),
            "http://localhost:3000/tracking/events"
        );
    }

    #[test]
    fn test_client_strips_trailing_slash() {
        let client = FgsServerClient::new("http://localhost:3000/");
        assert_eq!(client.base_url(), "http://localhost:3000");
    }
}
