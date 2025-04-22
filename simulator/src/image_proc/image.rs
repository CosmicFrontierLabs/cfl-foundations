use image::{GrayImage, Luma};
use ndarray::Array2;

/// Converts an ndarray Array2<u8> to an image::GrayImage
///
/// This function takes a 2D array of u8 values and converts it to a GrayImage
/// from the image crate, preserving the data arrangement. The conversion uses
/// a direct mapping where array indices [y, x] map to pixel coordinates (x, y).
/// Note that array dimensions are (height, width) while image dimensions are (width, height).
///
/// # Arguments
/// * `arr` - Reference to an Array2<u8> containing grayscale pixel values
///
/// # Returns
/// * A new GrayImage containing the same data as the input array
pub fn array2_to_gray_image(arr: &Array2<u8>) -> GrayImage {
    // Get array dimensions
    let (height, width) = arr.dim();

    // Create a new GrayImage
    let mut img = GrayImage::new(width as u32, height as u32);

    // Copy data from array to image
    for y in 0..height {
        for x in 0..width {
            img.put_pixel(x as u32, y as u32, Luma([arr[[y, x]]]));
        }
    }

    img
}
