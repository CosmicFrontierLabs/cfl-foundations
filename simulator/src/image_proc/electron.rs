use ndarray::Array2;

pub struct StarInFrame {
    pub x: f64,
    pub y: f64,
    pub flux: f64,
}

/// Adds stars to an image by approximating a Gaussian point spread function (PSF).
///
/// This function takes a mutable reference to an image and adds the flux contribution
/// of each star to the appropriate pixels based on a Gaussian PSF with the specified sigma.
///
/// # Arguments
/// * `image` - A mutable reference to the 2D array representing the image
/// * `stars` - A vector of StarInFrame objects containing position and flux information
/// * `sigma_pix` - The standard deviation of the Gaussian PSF in pixels
///
/// # Examples
/// ```
/// use ndarray::Array2;
/// use simulator::image_proc::electron::{add_stars_to_image, StarInFrame};
///
/// let mut image = Array2::zeros((100, 100));
/// let stars = vec![StarInFrame { x: 50.0, y: 50.0, flux: 1000.0 }];
/// add_stars_to_image(&mut image, stars, 2.0);
/// ```
pub fn add_stars_to_image(image: &mut Array2<f64>, stars: Vec<StarInFrame>, sigma_pix: f64) {
    // 4 std's is a good approximation for the PSF
    let max_pix_dist = (sigma_pix.max(1.0) * 4.0).ceil() as i32;

    let (width, height) = image.dim();
    let c = sigma_pix * sigma_pix * 2.0;

    let pre_term = 1.0 / (2.0 * sigma_pix * sigma_pix * std::f64::consts::PI);

    // Calculate the contribution of all stars to this pixel
    for star in &stars {
        // Calculate distance from star to pixel
        let xc = star.x.round() as i32;
        let yc = star.y.round() as i32;

        for y in (xc - max_pix_dist)..=(xc + max_pix_dist) {
            for x in (yc - max_pix_dist)..=(yc + max_pix_dist) {
                // Bounds check x/y - Skip out of bounds pixels
                if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
                    continue;
                }

                let dx = star.x - y as f64;
                let dy = star.y - x as f64;
                let distance_squared = dx * dx + dy * dy;
                // Update pixel value with total flux
                let contribution = star.flux * pre_term * (-distance_squared / c).exp();

                image[[x as usize, y as usize]] += contribution;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use rand::Rng;

    use super::*;

    #[test]
    fn test_add_star_total_flux() {
        let mut image = Array2::zeros((50, 50));
        let sigma_pix = 2.0;
        let total_flux = 1000.0;

        let stars = vec![StarInFrame {
            x: 25.0,
            y: 25.0,
            flux: total_flux,
        }];

        add_stars_to_image(&mut image, stars, sigma_pix);

        let added_flux = image.sum();
        assert_relative_eq!(added_flux, total_flux, epsilon = 0.1);
    }

    #[test]
    fn test_add_star_oob() {
        let mut image = Array2::zeros((50, 50));
        let sigma_pix = 2.0;
        let total_flux = 1000.0;

        let stars = vec![StarInFrame {
            x: 60.0,
            y: 60.0,
            flux: total_flux,
        }];

        add_stars_to_image(&mut image, stars, sigma_pix);

        let added_flux = image.sum();
        assert_relative_eq!(added_flux, 0.0, epsilon = 0.1);
    }

    #[test]
    fn test_add_star_edge() {
        let mut image = Array2::zeros((50, 50));
        let sigma_pix = 2.0;
        let total_flux = 1000.0;

        let stars = vec![StarInFrame {
            x: 0.5,
            y: 10.0,
            flux: total_flux,
        }];

        add_stars_to_image(&mut image, stars, sigma_pix);

        let added_flux = image.sum();

        // TODO(meawoppl) - tighten up image edge conventions pix vs. edge centered etc
        // Right now pixel coords are edge/corner centered, but flus is kinda not intuitive that way
        assert!(
            added_flux > 100.0,
            "Added flux is out of expected range: {}",
            added_flux
        );
    }

    #[test]
    fn test_add_four_stars_corners() {
        let mut image = Array2::zeros((50, 50));
        let sigma_pix = 2.0;
        let total_flux = 250.0;

        let stars = vec![
            StarInFrame {
                x: 0.0,
                y: 0.0,
                flux: total_flux,
            },
            StarInFrame {
                x: 0.0,
                y: 50.0,
                flux: total_flux,
            },
            StarInFrame {
                x: 50.0,
                y: 0.0,
                flux: total_flux,
            },
            StarInFrame {
                x: 50.0,
                y: 50.0,
                flux: total_flux,
            },
        ];

        add_stars_to_image(&mut image, stars, sigma_pix);

        let added_flux = image.sum();

        // Each of these should fall 3/4 off the image, resulting one flux worth
        assert_relative_eq!(added_flux, total_flux, epsilon = 1.0)
    }

    #[test]
    fn test_fuzz() {
        let mut image = Array2::zeros((50, 50));
        let sigma_pix = 2.0;
        let total_flux = 100.0;

        let mut rng = rand::thread_rng();

        let mut stars = Vec::new();
        for _ in 0..100 {
            stars.push(StarInFrame {
                x: rng.gen_range(-50.0..150.0),
                y: rng.gen_range(-50.0..150.0),
                flux: total_flux,
            });
        }

        add_stars_to_image(&mut image, stars, sigma_pix);

        let added_flux = image.sum();

        // Very loose bounds, but should catch egregious errors
        assert!(added_flux > 0.0);
    }
}
