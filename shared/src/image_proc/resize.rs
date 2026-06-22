//! Star-preserving image downsampling.
//!
//! Naive averaging dilutes point sources (stars) when shrinking an image —
//! a one-pixel star spread over an N×N block loses a factor of N² in peak
//! value. These routines downsample while keeping bright features intact, for
//! display rasters, thumbnails, and quick-look previews of star fields.
//!
//! Two flavors:
//! - [`star_centric_resize`] — fit within a target `max_size` (aspect-ratio
//!   preserved) using a windowed **percentile** (100 = max filter, best for
//!   star preservation; 50 = median, rejects hot pixels).
//! - [`downsample_max`] — the cheap fixed **integer-factor** block-max.
//!
//! Both are pure `ndarray` (no external filter dependency), matching the rest
//! of `image_proc`.

use ndarray::Array2;

/// Target dimensions that fit within `max_size` while preserving aspect ratio.
///
/// Returns `(new_height, new_width)`. If the image already fits, the original
/// dimensions are returned unchanged.
pub fn get_new_size(max_size: usize, height: usize, width: usize) -> (usize, usize) {
    if width > height && max_size < width {
        let new_width = max_size;
        let new_height = ((max_size as f64 / width as f64) * height as f64).round() as usize;
        (new_height, new_width)
    } else if max_size < height {
        let new_height = max_size;
        let new_width = ((max_size as f64 / height as f64) * width as f64).round() as usize;
        (new_height, new_width)
    } else {
        (height, width)
    }
}

/// Percentile of an already-gathered window (`values` need not be sorted).
/// `percentile` is in `[0, 100]`; 100 returns the max, 0 the min, 50 the median.
fn window_percentile(values: &mut [u16], percentile: f64) -> u16 {
    debug_assert!(!values.is_empty());
    values.sort_unstable();
    let n = values.len();
    let idx = ((percentile / 100.0) * (n - 1) as f64).round() as usize;
    values[idx.min(n - 1)]
}

/// Downsample a grayscale `u16` frame to fit within `max_size` (aspect ratio
/// preserved) using a windowed percentile, then decimating by striding.
///
/// `percentile` controls the filter applied over each downsample-sized window:
/// - `100.0` — max filter, preserves the brightest pixel (best for stars)
/// - `50.0` — median filter (rejects single hot pixels)
/// - `80.0` — a balance between signal preservation and smoothing
///
/// Returns the frame unchanged if it already fits within `max_size`.
pub fn star_centric_resize(frame: &Array2<u16>, max_size: usize, percentile: f64) -> Array2<u16> {
    let (height, width) = (frame.nrows(), frame.ncols());
    let (new_height, new_width) = get_new_size(max_size, height, width);

    if new_height >= height && new_width >= width {
        return frame.clone();
    }

    let factor = height.max(width).div_ceil(new_height.max(new_width)).max(1);
    let out_rows = height.div_ceil(factor);
    let out_cols = width.div_ceil(factor);
    let half = factor / 2;

    let mut result = Array2::<u16>::zeros((out_rows, out_cols));
    let mut window = Vec::with_capacity(factor * factor);
    for r in 0..out_rows {
        for c in 0..out_cols {
            // Window of `factor`x`factor` centered on the strided sample point,
            // clamped to the frame (out-of-bounds pixels are dropped).
            let (cr, cc) = (r * factor, c * factor);
            window.clear();
            for i in 0..factor {
                let rr = (cr + i).wrapping_sub(half);
                if rr >= height {
                    continue;
                }
                for j in 0..factor {
                    let cc2 = (cc + j).wrapping_sub(half);
                    if cc2 < width {
                        window.push(frame[[rr, cc2]]);
                    }
                }
            }
            result[[r, c]] = window_percentile(&mut window, percentile);
        }
    }
    result
}

/// Block-max downsample by an integer `factor` — the cheap, fixed-ratio sibling
/// of [`star_centric_resize`]. Each output pixel is the maximum over its
/// `factor`×`factor` block, so point sources survive where averaging would
/// dilute them.
///
/// Use this for a constant integer reduction (e.g. a display raster decimated
/// by a fixed factor); reach for [`star_centric_resize`] when you need to fit a
/// target `max_size`, preserve aspect ratio, or tune the percentile. Edge
/// pixels beyond the last whole block are dropped (output dims are
/// `floor(dim / factor)`).
pub fn downsample_max(frame: &Array2<u16>, factor: usize) -> Array2<u16> {
    let factor = factor.max(1);
    let (height, width) = (frame.nrows(), frame.ncols());
    let (out_rows, out_cols) = (height / factor, width / factor);
    let mut out = Array2::<u16>::zeros((out_rows, out_cols));
    for r in 0..out_rows {
        for c in 0..out_cols {
            let mut m = 0u16;
            for i in 0..factor {
                for j in 0..factor {
                    m = m.max(frame[[r * factor + i, c * factor + j]]);
                }
            }
            out[[r, c]] = m;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_new_size_wider() {
        assert_eq!(get_new_size(512, 1000, 2000), (256, 512));
    }

    #[test]
    fn get_new_size_taller() {
        assert_eq!(get_new_size(512, 2000, 1000), (512, 256));
    }

    #[test]
    fn get_new_size_already_fits() {
        assert_eq!(get_new_size(512, 100, 200), (100, 200));
    }

    #[test]
    fn resize_no_op_when_small() {
        let frame = Array2::<u16>::zeros((64, 64));
        assert_eq!(star_centric_resize(&frame, 128, 100.0).dim(), (64, 64));
    }

    #[test]
    fn resize_downsamples_to_within_max() {
        let frame = Array2::<u16>::zeros((1000, 1000));
        let out = star_centric_resize(&frame, 512, 100.0);
        assert!(out.nrows() <= 512 && out.ncols() <= 512);
        assert!(out.nrows() < 1000);
    }

    #[test]
    fn resize_max_filter_preserves_bright_pixel() {
        let mut frame = Array2::<u16>::zeros((128, 128));
        frame[[10, 10]] = 60000;
        let out = star_centric_resize(&frame, 64, 100.0);
        assert_eq!(*out.iter().max().unwrap(), 60000);
    }

    #[test]
    fn resize_median_filter_rejects_outlier() {
        let mut frame = Array2::from_elem((128, 128), 1000u16);
        frame[[64, 64]] = 60000;
        let out = star_centric_resize(&frame, 64, 50.0);
        assert!(*out.iter().max().unwrap() < 60000);
    }

    #[test]
    fn downsample_max_dimensions_and_star_preserved() {
        let mut frame = Array2::<u16>::zeros((256, 256));
        frame[[33, 70]] = 55000;
        let out = downsample_max(&frame, 4);
        assert_eq!(out.dim(), (64, 64));
        assert_eq!(*out.iter().max().unwrap(), 55000);
        assert_eq!(out[[33 / 4, 70 / 4]], 55000);
    }

    #[test]
    fn downsample_max_factor_one_keeps_size() {
        let frame = Array2::<u16>::from_elem((10, 12), 7u16);
        assert_eq!(downsample_max(&frame, 1).dim(), (10, 12));
    }
}
