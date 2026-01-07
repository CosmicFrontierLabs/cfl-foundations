//! Optical calibration pattern that receives tracking feedback via ZMQ.
//!
//! Displays a spot at grid positions, uses incoming tracking measurements to estimate
//! the affine transformation (scale, rotation, translation) between display and sensor
//! coordinate systems.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use shared::image_size::PixelShape;
use shared::optical_alignment::{estimate_affine_transform, OpticalAlignment, PointCorrespondence};
use shared::tracking_collector::TrackingCollector;

use super::shared::{compute_normalization_factor, render_gaussian_spot, BlendMode};

/// Default number of measurements to average per position
const DEFAULT_MEASUREMENTS_PER_POSITION: usize = 30;

/// Parameters for rendering a Gaussian spot.
#[derive(Debug, Clone, Copy)]
pub struct SpotParams {
    pub x: f64,
    pub y: f64,
    pub fwhm_pixels: f64,
    pub normalization_factor: f64,
}

/// Generate spots arranged in a circle centered on the display.
fn generate_circle(
    num_points: usize,
    radius_pixels: f64,
    display_size: PixelShape,
    fwhm_pixels: f64,
) -> Vec<SpotParams> {
    let center_x = display_size.width as f64 / 2.0;
    let center_y = display_size.height as f64 / 2.0;
    let normalization_factor = compute_normalization_factor(fwhm_pixels, 255.0);

    (0..num_points)
        .map(|i| {
            let angle = 2.0 * std::f64::consts::PI * i as f64 / num_points as f64;
            SpotParams {
                x: center_x + radius_pixels * angle.cos(),
                y: center_y + radius_pixels * angle.sin(),
                fwhm_pixels,
                normalization_factor,
            }
        })
        .collect()
}

/// Generate a centered grid of spot parameters.
///
/// Creates an NxN grid of spots centered on the display with the given spacing between adjacent points.
fn generate_centered_grid(
    grid_size: usize,
    grid_spacing: f64,
    display_size: PixelShape,
    fwhm_pixels: f64,
) -> Vec<SpotParams> {
    let center_x = display_size.width as f64 / 2.0;
    let center_y = display_size.height as f64 / 2.0;
    let normalization_factor = compute_normalization_factor(fwhm_pixels, 255.0);
    let half_extent = (grid_size - 1) as f64 / 2.0;

    let mut spots = Vec::with_capacity(grid_size * grid_size);
    for row in 0..grid_size {
        for col in 0..grid_size {
            let offset_x = (col as f64 - half_extent) * grid_spacing;
            let offset_y = (row as f64 - half_extent) * grid_spacing;
            spots.push(SpotParams {
                x: center_x + offset_x,
                y: center_y + offset_y,
                fwhm_pixels,
                normalization_factor,
            });
        }
    }
    spots
}

/// Runs an optical calibration sequence by displaying spots at known positions
/// and collecting sensor feedback to estimate the display-to-sensor affine transform.
pub struct CalibrationRunner {
    /// Tracking collector receiving measurements from the sensor
    collector: TrackingCollector,
    /// Collected display→sensor point correspondences used for transform estimation
    calibration_points: Vec<PointCorrespondence>,
    /// Sensor measurements collected at the current display position (averaged before use)
    current_position_measurements: Vec<(f64, f64)>,
    /// Number of measurements to average per position before advancing
    measurements_per_position: usize,
    /// Sequence of display positions to visit during calibration
    spots: Vec<SpotParams>,
    /// Index of current position in the spots sequence
    spot_index: usize,
    /// Timestamp of last position change (used for settle timing)
    last_move_time: Instant,
    /// Duration to wait after position change before accepting measurements
    settle_duration: Duration,
}

impl CalibrationRunner {
    /// Create a calibration runner with a centered grid pattern.
    ///
    /// # Arguments
    /// * `collector` - TrackingCollector for receiving tracking messages
    /// * `grid_size` - Number of points per axis (e.g., 5 gives 25 points)
    /// * `grid_spacing` - Distance in pixels between adjacent grid points
    /// * `display_size` - Display dimensions in pixels
    /// * `spot_fwhm_pixels` - FWHM of the spot in pixels
    /// * `settle_duration` - Time to wait after each position change before accepting measurements
    pub fn for_grid(
        collector: TrackingCollector,
        grid_size: usize,
        grid_spacing: f64,
        display_size: PixelShape,
        spot_fwhm_pixels: f64,
        settle_duration: Duration,
    ) -> Self {
        let spots = generate_centered_grid(grid_size, grid_spacing, display_size, spot_fwhm_pixels);
        Self::from_spots(collector, spots, settle_duration)
    }

    /// Create a calibration runner with points arranged in a circle.
    ///
    /// # Arguments
    /// * `collector` - TrackingCollector for receiving tracking messages
    /// * `num_points` - Number of points around the circle
    /// * `radius_pixels` - Radius of the circle in pixels from display center
    /// * `display_size` - Display dimensions in pixels
    /// * `spot_fwhm_pixels` - FWHM of the spot in pixels
    /// * `settle_duration` - Time to wait after each position change before accepting measurements
    pub fn for_circle(
        collector: TrackingCollector,
        num_points: usize,
        radius_pixels: f64,
        display_size: PixelShape,
        spot_fwhm_pixels: f64,
        settle_duration: Duration,
    ) -> Self {
        let spots = generate_circle(num_points, radius_pixels, display_size, spot_fwhm_pixels);
        Self::from_spots(collector, spots, settle_duration)
    }

    /// Create a calibration runner from a pre-computed spot sequence.
    fn from_spots(
        collector: TrackingCollector,
        spots: Vec<SpotParams>,
        settle_duration: Duration,
    ) -> Self {
        let num_spots = spots.len();
        Self {
            collector,
            calibration_points: Vec::with_capacity(num_spots),
            current_position_measurements: Vec::with_capacity(DEFAULT_MEASUREMENTS_PER_POSITION),
            measurements_per_position: DEFAULT_MEASUREMENTS_PER_POSITION,
            spots,
            spot_index: 0,
            last_move_time: Instant::now(),
            settle_duration,
        }
    }

    /// Check if the calibration is complete (all positions visited).
    pub fn is_calibration_complete(&self) -> bool {
        self.spot_index >= self.spots.len()
    }

    /// Check if current position has settled (ready for measurements).
    fn is_settled(&self) -> bool {
        !self.is_calibration_complete() && self.last_move_time.elapsed() >= self.settle_duration
    }

    /// Poll for new tracking messages and add calibration points.
    ///
    /// Only accepts measurements when position is stable.
    /// Collects measurements, averages them, and advances to next position.
    pub fn poll_tracking_messages(&mut self) {
        let messages = self.collector.poll();

        if !self.is_settled() {
            return;
        }

        // Collect sensor measurements at current position
        for tracking_msg in messages {
            self.current_position_measurements
                .push((tracking_msg.x, tracking_msg.y));
        }

        // Check if we have enough measurements to average and advance
        if self.current_position_measurements.len() >= self.measurements_per_position {
            let n = self.current_position_measurements.len() as f64;
            let (sum_x, sum_y) = self
                .current_position_measurements
                .iter()
                .fold((0.0, 0.0), |(ax, ay), (x, y)| (ax + x, ay + y));
            let avg_sensor_x = sum_x / n;
            let avg_sensor_y = sum_y / n;

            let spot = self.spots[self.spot_index];
            let correspondence =
                PointCorrespondence::new(spot.x, spot.y, avg_sensor_x, avg_sensor_y);

            println!(
                "Position ({:.1}, {:.1}) -> sensor ({:.2}, {:.2}) [{} measurements averaged]",
                spot.x,
                spot.y,
                avg_sensor_x,
                avg_sensor_y,
                self.current_position_measurements.len()
            );

            self.calibration_points.push(correspondence);

            self.current_position_measurements.clear();

            // Advance to next position
            self.spot_index += 1;
            self.last_move_time = Instant::now();
        }
    }

    /// Estimate the affine transform from collected calibration points.
    ///
    /// Returns `None` if estimation fails (need at least 3 points for affine transform).
    pub fn estimate_transform(&self) -> Option<OpticalAlignment> {
        estimate_affine_transform(&self.calibration_points)
    }

    /// Get current spot parameters.
    /// Returns `None` after calibration is complete.
    pub fn current_spot(&self) -> Option<SpotParams> {
        if self.is_calibration_complete() {
            None
        } else {
            Some(self.spots[self.spot_index])
        }
    }

    /// Get number of calibration points collected
    pub fn num_calibration_points(&self) -> usize {
        self.calibration_points.len()
    }
}

pub fn generate_into_buffer(buffer: &mut [u8], size: PixelShape, state: &Mutex<CalibrationRunner>) {
    buffer.fill(0);

    let mut state = state.lock().unwrap();
    state.poll_tracking_messages();

    if let Some(spot) = state.current_spot() {
        render_gaussian_spot(
            buffer,
            size.width as u32,
            size.height as u32,
            spot.x,
            spot.y,
            spot.fwhm_pixels,
            spot.normalization_factor,
            BlendMode::Additive,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use shared::camera_interface::Timestamp;
    use shared::image_proc::centroid::SpotShape;
    use shared::tracking_message::TrackingMessage;
    use shared::zmq::{TypedZmqPublisher, TypedZmqSubscriber};

    fn test_shape() -> SpotShape {
        SpotShape {
            flux: 1000.0,
            m_xx: 2.0,
            m_yy: 2.0,
            m_xy: 0.0,
            aspect_ratio: 1.0,
            diameter: 4.0,
        }
    }

    #[test]
    fn test_generate_centered_grid() {
        let display_size = PixelShape {
            width: 100,
            height: 100,
        };
        let grid_size = 3;
        let grid_spacing = 10.0;
        let fwhm = 2.0;

        let spots = generate_centered_grid(grid_size, grid_spacing, display_size, fwhm);

        // 3x3 grid = 9 spots
        assert_eq!(spots.len(), 9);

        // Center of display is at (50, 50)
        // With grid_size=3 and spacing=10, half_extent = 1.0
        // Offsets are: -10, 0, 10 in both x and y
        let expected_positions = [
            (40.0, 40.0),
            (50.0, 40.0),
            (60.0, 40.0),
            (40.0, 50.0),
            (50.0, 50.0),
            (60.0, 50.0),
            (40.0, 60.0),
            (50.0, 60.0),
            (60.0, 60.0),
        ];

        for (i, (expected_x, expected_y)) in expected_positions.iter().enumerate() {
            assert_relative_eq!(spots[i].x, expected_x, epsilon = 1e-10);
            assert_relative_eq!(spots[i].y, expected_y, epsilon = 1e-10);
            assert_relative_eq!(spots[i].fwhm_pixels, fwhm, epsilon = 1e-10);
        }

        // Test single point grid (edge case)
        let single_spot = generate_centered_grid(1, 10.0, display_size, fwhm);
        assert_eq!(single_spot.len(), 1);
        assert_relative_eq!(single_spot[0].x, 50.0, epsilon = 1e-10);
        assert_relative_eq!(single_spot[0].y, 50.0, epsilon = 1e-10);
    }

    #[test]
    fn test_generate_circle() {
        let display_size = PixelShape {
            width: 100,
            height: 100,
        };
        let num_points = 4;
        let radius = 20.0;
        let fwhm = 2.0;

        let spots = generate_circle(num_points, radius, display_size, fwhm);

        // 4 points around a circle
        assert_eq!(spots.len(), 4);

        // Center is at (50, 50), radius 20
        // Point 0: angle 0 -> (50+20, 50) = (70, 50)
        // Point 1: angle pi/2 -> (50, 50+20) = (50, 70)
        // Point 2: angle pi -> (50-20, 50) = (30, 50)
        // Point 3: angle 3pi/2 -> (50, 50-20) = (50, 30)
        assert_relative_eq!(spots[0].x, 70.0, epsilon = 1e-10);
        assert_relative_eq!(spots[0].y, 50.0, epsilon = 1e-10);
        assert_relative_eq!(spots[1].x, 50.0, epsilon = 1e-10);
        assert_relative_eq!(spots[1].y, 70.0, epsilon = 1e-10);
        assert_relative_eq!(spots[2].x, 30.0, epsilon = 1e-10);
        assert_relative_eq!(spots[2].y, 50.0, epsilon = 1e-10);
        assert_relative_eq!(spots[3].x, 50.0, epsilon = 1e-10);
        assert_relative_eq!(spots[3].y, 30.0, epsilon = 1e-10);

        // All points should have same FWHM
        for spot in &spots {
            assert_relative_eq!(spot.fwhm_pixels, fwhm, epsilon = 1e-10);
        }
    }

    /// Simulated affine transform parameters for testing.
    /// Represents: scale=0.5, rotation=5 degrees, translation=(100, 50)
    fn simulated_transform(display_x: f64, display_y: f64) -> (f64, f64) {
        let scale = 0.5;
        let angle_rad = 5.0_f64.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();
        let tx = 100.0;
        let ty = 50.0;

        let sensor_x = scale * (cos_a * display_x - sin_a * display_y) + tx;
        let sensor_y = scale * (sin_a * display_x + cos_a * display_y) + ty;
        (sensor_x, sensor_y)
    }

    #[test]
    fn test_optical_calibration_state_with_zmq() {
        let ctx = zmq::Context::new();

        // Create and bind publisher
        let pub_socket = ctx.socket(zmq::PUB).unwrap();
        pub_socket.bind("tcp://127.0.0.1:*").unwrap();
        let endpoint = pub_socket.get_last_endpoint().unwrap().unwrap();
        let publisher = TypedZmqPublisher::<TrackingMessage>::new(pub_socket);

        // Create and connect subscriber wrapped in TrackingCollector
        let sub_socket = ctx.socket(zmq::SUB).unwrap();
        sub_socket.connect(&endpoint).unwrap();
        sub_socket.set_subscribe(b"").unwrap();
        let subscriber = TypedZmqSubscriber::<TrackingMessage>::new(sub_socket);
        let collector = TrackingCollector::new(subscriber);

        // Wait for ZMQ slow joiner
        std::thread::sleep(Duration::from_millis(100));

        // Create calibration state with 3x3 grid, zero settle time for testing
        let display_size = PixelShape {
            width: 1024,
            height: 1024,
        };
        let mut state = CalibrationRunner::for_grid(
            collector,
            3,     // 3x3 = 9 points
            200.0, // grid spacing in pixels
            display_size,
            5.0,            // fwhm
            Duration::ZERO, // no settle delay for test
        );

        // Run calibration loop
        let mut iterations = 0;
        let max_iterations = 1000;

        while !state.is_calibration_complete() && iterations < max_iterations {
            iterations += 1;

            // Get current display position
            let spot = state.current_spot().expect("should have position");
            let (display_x, display_y) = (spot.x, spot.y);

            // Compute simulated sensor position
            let (sensor_x, sensor_y) = simulated_transform(display_x, display_y);

            // Send enough measurements for this position
            for i in 0..35 {
                let msg = TrackingMessage::new(
                    1,
                    sensor_x + (i as f64 * 0.001), // tiny noise
                    sensor_y + (i as f64 * 0.001),
                    Timestamp::new(iterations as u64, 0),
                    test_shape(),
                );
                publisher.send(&msg).unwrap();
            }

            // Give messages time to arrive
            std::thread::sleep(Duration::from_millis(10));

            // Poll to process messages
            state.poll_tracking_messages();
        }

        assert!(
            state.is_calibration_complete(),
            "Calibration should complete within {} iterations",
            max_iterations
        );
        assert_eq!(state.num_calibration_points(), 9);

        // Estimate transform
        let alignment = state
            .estimate_transform()
            .expect("should estimate transform with 9 points");

        // Verify transform parameters match expected
        // Expected: scale=0.5, rotation=5 degrees, translation=(100, 50)
        let (scale_x, scale_y) = alignment.scale();
        let rotation_deg = alignment.rotation_degrees();

        println!("Recovered transform:");
        println!("  Scale: ({:.4}, {:.4})", scale_x, scale_y);
        println!("  Rotation: {:.4} degrees", rotation_deg);
        println!("  Translation: ({:.4}, {:.4})", alignment.tx, alignment.ty);
        println!("  RMS error: {:?}", alignment.rms_error);

        // Check scale is approximately 0.5
        assert_relative_eq!(scale_x, 0.5, epsilon = 0.01);
        assert_relative_eq!(scale_y, 0.5, epsilon = 0.01);

        // Check rotation is approximately 5 degrees
        assert_relative_eq!(rotation_deg, 5.0, epsilon = 0.5);

        // Check translation is approximately (100, 50)
        assert_relative_eq!(alignment.tx, 100.0, epsilon = 1.0);
        assert_relative_eq!(alignment.ty, 50.0, epsilon = 1.0);

        // RMS error should be very small (just tiny noise)
        assert!(
            alignment.rms_error.unwrap() < 1e-8,
            "RMS error should be small, got {}",
            alignment.rms_error.unwrap()
        );
    }
}
