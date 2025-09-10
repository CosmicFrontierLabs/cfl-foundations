//! MONACLE - Modular Orientation, Navigation & Optical Control Logic Engine
//!
//! Fine Guidance System state machine implementation based on the FGS ConOps.
//! Processes images through states: Idle -> Acquiring -> Calibrating -> Tracking

use ndarray::ArrayView2;
use serde::{Deserialize, Serialize};
use std::time::Instant;

/// ROI (Region of Interest) around a guide star
#[derive(Debug, Clone)]
pub struct Roi {
    /// Center X position in full frame
    pub center_x: f64,
    /// Center Y position in full frame
    pub center_y: f64,
    /// Width of ROI in pixels
    pub width: usize,
    /// Height of ROI in pixels  
    pub height: usize,
    /// Reference centroid position (from calibration)
    pub reference_x: f64,
    /// Reference centroid position (from calibration)
    pub reference_y: f64,
}

/// A selected guide star
#[derive(Debug, Clone)]
pub struct GuideStar {
    /// Unique identifier
    pub id: usize,
    /// Position in full frame
    pub x: f64,
    pub y: f64,
    /// Estimated flux
    pub flux: f64,
    /// Signal-to-noise ratio
    pub snr: f64,
    /// Region of interest for tracking
    pub roi: Roi,
}

/// Guidance update produced by the system
#[derive(Debug, Clone)]
pub struct GuidanceUpdate {
    /// Computed X error in pixels
    pub delta_x: f64,
    /// Computed Y error in pixels
    pub delta_y: f64,
    /// Number of guide stars used
    pub num_stars_used: usize,
    /// Timestamp of update
    pub timestamp: Instant,
    /// Quality metric (0.0 to 1.0)
    pub quality: f64,
}

impl Default for GuidanceUpdate {
    fn default() -> Self {
        Self {
            delta_x: 0.0,
            delta_y: 0.0,
            num_stars_used: 0,
            timestamp: Instant::now(),
            quality: 0.0,
        }
    }
}

/// Fine Guidance System states
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FgsState {
    /// Waiting for START_FGS command
    Idle,
    /// Collecting frames for averaging
    Acquiring { frames_collected: usize },
    /// Detecting stars, selecting guides, setting references
    Calibrating,
    /// Continuous centroiding and FSM commanding
    Tracking { frames_processed: usize },
    /// Attempting to recover lost stars
    Reacquiring { attempts: usize },
}

/// Events that trigger state transitions
#[derive(Debug, Clone)]
pub enum FgsEvent<'a> {
    /// Start the FGS
    StartFgs,
    /// Abort current operation
    Abort,
    /// Stop FGS (graceful shutdown)
    StopFgs,
    /// Process a new image frame
    ProcessFrame(ArrayView2<'a, u16>),
}

/// Configuration for the Fine Guidance System
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FgsConfig {
    /// Number of frames to average during acquisition
    pub acquisition_frames: usize,
    /// Minimum SNR for guide star selection
    pub min_guide_star_snr: f64,
    /// Maximum number of guide stars to track
    pub max_guide_stars: usize,
    /// ROI size around each guide star (pixels)
    pub roi_size: usize,
    /// Maximum reacquisition attempts before recalibration
    pub max_reacquisition_attempts: usize,
    /// Centroid computation method
    pub centroid_method: CentroidMethod,
}

impl Default for FgsConfig {
    fn default() -> Self {
        Self {
            acquisition_frames: 10,
            min_guide_star_snr: 20.0,
            max_guide_stars: 3,
            roi_size: 64,
            max_reacquisition_attempts: 5,
            centroid_method: CentroidMethod::CenterOfMass,
        }
    }
}

/// Centroid computation methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CentroidMethod {
    /// Intensity-weighted center of mass
    CenterOfMass,
    /// Gaussian PSF fitting
    GaussianFit,
    /// Quadratic interpolation
    QuadraticInterpolation,
}

/// Main Fine Guidance System state machine
pub struct FineGuidanceSystem {
    /// Current state
    state: FgsState,
    /// System configuration
    config: FgsConfig,
    /// Selected guide stars (populated during calibration)
    guide_stars: Vec<GuideStar>,
    /// Accumulated frames during acquisition
    accumulated_frames: Vec<ArrayView2<'static, u16>>,
    /// Last guidance update
    last_update: Option<GuidanceUpdate>,
}

impl FineGuidanceSystem {
    /// Create a new Fine Guidance System
    pub fn new(config: FgsConfig) -> Self {
        Self {
            state: FgsState::Idle,
            config,
            guide_stars: Vec::new(),
            accumulated_frames: Vec::new(),
            last_update: None,
        }
    }

    /// Process an event and potentially transition states
    pub fn process_event(&mut self, event: FgsEvent<'_>) -> Result<Option<GuidanceUpdate>, String> {
        use FgsState::*;

        let new_state = match (&self.state, event) {
            // From Idle
            (Idle, FgsEvent::StartFgs) => {
                log::info!("Starting FGS, entering Acquiring state");
                self.accumulated_frames.clear();
                self.guide_stars.clear();
                Acquiring {
                    frames_collected: 0,
                }
            }

            // From Acquiring
            (Acquiring { frames_collected }, FgsEvent::ProcessFrame(frame)) => {
                let frames = frames_collected + 1;
                self.accumulate_frame(frame)?;

                if frames >= self.config.acquisition_frames {
                    log::info!("Acquisition complete, entering Calibrating state");
                    Calibrating
                } else {
                    Acquiring {
                        frames_collected: frames,
                    }
                }
            }
            (Acquiring { .. }, FgsEvent::Abort) => {
                log::info!("Aborting acquisition, returning to Idle");
                self.accumulated_frames.clear();
                Idle
            }

            // From Calibrating
            (Calibrating, FgsEvent::ProcessFrame(frame)) => {
                self.calibrate(frame)?;

                if !self.guide_stars.is_empty() {
                    log::info!(
                        "Calibration complete with {} guide stars, entering Tracking",
                        self.guide_stars.len()
                    );
                    Tracking {
                        frames_processed: 0,
                    }
                } else {
                    log::warn!("No suitable guide stars found, returning to Idle");
                    Idle
                }
            }

            // From Tracking
            (Tracking { frames_processed }, FgsEvent::ProcessFrame(frame)) => {
                let frames = *frames_processed;
                let update = self.track(frame)?;

                if update.num_stars_used > 0 {
                    self.last_update = Some(update.clone());
                    Tracking {
                        frames_processed: frames + 1,
                    }
                } else {
                    log::warn!("Lost all guide stars, entering Reacquiring");
                    Reacquiring { attempts: 0 }
                }
            }
            (Tracking { .. }, FgsEvent::StopFgs) => {
                log::info!("Stopping FGS, returning to Idle");
                Idle
            }

            // From Reacquiring
            (Reacquiring { attempts }, FgsEvent::ProcessFrame(frame)) => {
                let attempt_count = *attempts;
                let recovered = self.attempt_reacquisition(frame)?;

                if recovered {
                    log::info!("Lock recovered, returning to Tracking");
                    Tracking {
                        frames_processed: 0,
                    }
                } else if attempt_count + 1 >= self.config.max_reacquisition_attempts {
                    log::warn!("Reacquisition timeout, returning to Calibrating");
                    Calibrating
                } else {
                    Reacquiring {
                        attempts: attempt_count + 1,
                    }
                }
            }
            (Reacquiring { .. }, FgsEvent::Abort) => {
                log::info!("Aborting reacquisition, returning to Idle");
                Idle
            }

            // Invalid transitions
            _ => {
                log::warn!("Invalid state transition");
                self.state.clone()
            }
        };

        self.state = new_state;
        Ok(self.last_update.clone())
    }

    /// Process a single image frame
    pub fn process_frame(
        &mut self,
        frame: ArrayView2<u16>,
    ) -> Result<Option<GuidanceUpdate>, String> {
        self.process_event(FgsEvent::ProcessFrame(frame))
    }

    /// Get the current state
    pub fn state(&self) -> &FgsState {
        &self.state
    }

    /// Accumulate frames during acquisition
    fn accumulate_frame(&mut self, _frame: ArrayView2<u16>) -> Result<(), String> {
        // TODO: Store frame for averaging
        Ok(())
    }

    /// Perform calibration: detect stars, select guides, set references
    fn calibrate(&mut self, _frame: ArrayView2<u16>) -> Result<(), String> {
        // TODO: Implement calibration logic
        // 1. Average accumulated frames
        // 2. Detect all stars
        // 3. Select guide stars based on criteria
        // 4. Define ROIs
        // 5. Store reference positions
        Ok(())
    }

    /// Track guide stars and compute guidance update
    fn track(&mut self, _frame: ArrayView2<u16>) -> Result<GuidanceUpdate, String> {
        // TODO: Implement tracking logic
        // 1. Extract ROIs
        // 2. Compute centroids
        // 3. Calculate deltas from reference
        // 4. Combine into guidance update
        Ok(GuidanceUpdate {
            delta_x: 0.0,
            delta_y: 0.0,
            num_stars_used: self.guide_stars.len(),
            timestamp: Instant::now(),
            quality: 1.0,
        })
    }

    /// Attempt to reacquire lost guide stars
    fn attempt_reacquisition(&mut self, _frame: ArrayView2<u16>) -> Result<bool, String> {
        // TODO: Implement reacquisition logic
        // 1. Search in expanded ROIs
        // 2. Try to match with known guide stars
        // 3. Return true if enough stars recovered
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::Array2;

    #[test]
    fn test_state_transitions() {
        let mut fgs = FineGuidanceSystem::new(FgsConfig::default());

        // Should start in Idle
        assert_eq!(fgs.state(), &FgsState::Idle);

        // Start FGS
        let _ = fgs.process_event(FgsEvent::StartFgs);
        assert!(matches!(fgs.state(), FgsState::Acquiring { .. }));
    }

    #[test]
    fn test_process_frame() {
        let mut fgs = FineGuidanceSystem::new(FgsConfig::default());
        let dummy_frame = Array2::<u16>::zeros((100, 100));

        // Should do nothing in Idle state
        let result = fgs.process_frame(dummy_frame.view());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
