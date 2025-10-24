use image::{ImageBuffer, Rgb};

pub fn generate(width: u32, height: u32, level: u8) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    ImageBuffer::from_pixel(width, height, Rgb([level, level, level]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uniform_pattern_generation() {
        let img = generate(100, 100, 128);
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 100);

        let pixel = img.get_pixel(50, 50);
        assert_eq!(*pixel, Rgb([128, 128, 128]));

        let corner_pixel = img.get_pixel(0, 0);
        assert_eq!(*corner_pixel, Rgb([128, 128, 128]));
    }

    #[test]
    fn test_uniform_black() {
        let img = generate(100, 100, 0);
        let pixel = img.get_pixel(50, 50);
        assert_eq!(*pixel, Rgb([0, 0, 0]));
    }

    #[test]
    fn test_uniform_white() {
        let img = generate(100, 100, 255);
        let pixel = img.get_pixel(50, 50);
        assert_eq!(*pixel, Rgb([255, 255, 255]));
    }
}
