use ndarray::Array2;

use crate::image_proc::{
    convolve2d, detect_stars, gaussian_kernel, otsu_threshold, ConvolveMode, ConvolveOptions,
};

use super::StarDetection;

pub fn do_detections(
    sensor_image: &Array2<u16>,
    smooth_by: Option<f64>,
    threshold: Option<f64>,
) -> Vec<StarDetection> {
    // Cast it into float space
    let image_array = sensor_image.mapv(|x| x as f64);

    let smoothed = match smooth_by {
        Some(smooth) => {
            println!("Smoothing image with Gaussian kernel of size {} px", smooth);

            // TODO(meawoppl) - make this a function of erf() + round up kernel to nearest odd multiple
            let kernel_size = 9;
            let kernel = gaussian_kernel(kernel_size, smooth);

            println!("Convolving image with Gaussian kernel...");
            convolve2d(
                &image_array.view(),
                &kernel.view(),
                Some(ConvolveOptions {
                    mode: ConvolveMode::Same,
                }),
            )
        }
        None => {
            println!("Skipping smoothing step");
            image_array.clone()
        }
    };

    // Use the supplied threshold if provided, otherwise calculate Otsu's threshold
    let cutoff = match threshold {
        Some(t) => {
            println!("Using provided threshold: {:.6}", t);
            t
        }
        None => {
            let threshold = otsu_threshold(&smoothed.view());
            println!("Otsu's threshold: {:.6}", threshold);
            threshold
        }
    };

    println!("Detecting stars with cutoff: {:.6}", cutoff);
    // Detect stars using our new centroid-based detection
    detect_stars(&smoothed.view(), Some(cutoff))
}
