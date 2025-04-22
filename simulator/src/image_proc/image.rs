use image::{GrayImage, Luma, Rgb, RgbImage};
use ndarray::{Array2, ArrayView2};

// Method 1: Converting Array2<u8> to a grayscale image
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
