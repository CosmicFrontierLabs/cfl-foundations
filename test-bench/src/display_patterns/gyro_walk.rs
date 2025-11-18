use image::RgbImage;
use rayon::prelude::*;
use simulator::hardware::gyro::models::exail_astrix_ns;
use simulator::hardware::gyro::GyroModel;
use std::sync::Mutex;
use std::time::Duration;

pub struct GyroWalkState {
    gyro: GyroModel,
    reset_interval: Duration,
    last_reset_time: Duration,
    pixel_size_um: f64,
    focal_length_mm: f64,
    motion_scale: f64,
}

impl GyroWalkState {
    pub fn new(pixel_size_um: f64, focal_length_mm: f64, motion_scale: f64) -> Self {
        Self {
            gyro: exail_astrix_ns(),
            reset_interval: Duration::from_secs(5),
            last_reset_time: Duration::ZERO,
            pixel_size_um,
            focal_length_mm,
            motion_scale,
        }
    }

    pub fn update(&mut self, elapsed: Duration, frame_interval: Duration) {
        let time_since_reset = elapsed - self.last_reset_time;

        if time_since_reset >= self.reset_interval {
            self.gyro.reset();
            self.last_reset_time = elapsed;
        }

        self.gyro.step(frame_interval);
    }

    pub fn get_offset_pixels(&self) -> (f64, f64) {
        let (x_rad, y_rad, _z_rad) = self.gyro.angle_errors();

        let focal_length_m = self.focal_length_mm / 1000.0;
        let pixel_size_m = self.pixel_size_um / 1_000_000.0;

        let x_pixels = (x_rad * focal_length_m) / pixel_size_m * self.motion_scale;
        let y_pixels = (y_rad * focal_length_m) / pixel_size_m * self.motion_scale;

        (x_pixels, y_pixels)
    }
}

fn bilinear_sample_wraparound(base_img: &RgbImage, x: f64, y: f64) -> [u8; 3] {
    let img_width = base_img.width() as i32;
    let img_height = base_img.height() as i32;

    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;

    let fx = x - x0 as f64;
    let fy = y - y0 as f64;

    let wraparound = |coord: i32, size: i32| -> u32 {
        let mut wrapped = coord % size;
        if wrapped < 0 {
            wrapped += size;
        }
        wrapped as u32
    };

    let x0_safe = wraparound(x0, img_width);
    let x1_safe = wraparound(x1, img_width);
    let y0_safe = wraparound(y0, img_height);
    let y1_safe = wraparound(y1, img_height);

    let p00 = base_img.get_pixel(x0_safe, y0_safe);
    let p10 = base_img.get_pixel(x1_safe, y0_safe);
    let p01 = base_img.get_pixel(x0_safe, y1_safe);
    let p11 = base_img.get_pixel(x1_safe, y1_safe);

    let mut result = [0u8; 3];
    for c in 0..3 {
        let v00 = p00[c] as f64;
        let v10 = p10[c] as f64;
        let v01 = p01[c] as f64;
        let v11 = p11[c] as f64;

        let v0 = v00 * (1.0 - fx) + v10 * fx;
        let v1 = v01 * (1.0 - fx) + v11 * fx;
        let v = v0 * (1.0 - fy) + v1 * fy;

        result[c] = v.round().clamp(0.0, 255.0) as u8;
    }
    result
}

pub fn generate_into_buffer(
    buffer: &mut [u8],
    width: u32,
    height: u32,
    base_img: &RgbImage,
    gyro_state: &Mutex<GyroWalkState>,
    elapsed: Duration,
    frame_interval: Duration,
) {
    let mut state = gyro_state.lock().unwrap();
    state.update(elapsed, frame_interval);
    let (x_pixels, y_pixels) = state.get_offset_pixels();
    drop(state);

    println!("Center field offset: x={x_pixels:7.2} px, y={y_pixels:7.2} px");

    buffer.fill(0);

    let img_width = base_img.width();
    let img_height = base_img.height();
    let center_x = width as f64 / 2.0;
    let center_y = height as f64 / 2.0;

    buffer
        .par_chunks_mut((width * 3) as usize)
        .enumerate()
        .for_each(|(dst_y, row)| {
            for dst_x in 0..width {
                let src_x = dst_x as f64 - center_x - x_pixels + img_width as f64 / 2.0;
                let src_y = dst_y as f64 - center_y - y_pixels + img_height as f64 / 2.0;

                let pixel = bilinear_sample_wraparound(base_img, src_x, src_y);
                let idx = (dst_x * 3) as usize;
                row[idx] = pixel[0];
                row[idx + 1] = pixel[1];
                row[idx + 2] = pixel[2];
            }
        });
}
