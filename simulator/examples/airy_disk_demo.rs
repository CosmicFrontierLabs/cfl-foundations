use plotters::prelude::*;
use simulator::image_proc::AiryDisk;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Airy Disk Approximation Demonstration");
    println!("====================================");
    println!();

    // Create an Airy disk for typical astronomical wavelength and aperture
    let wavelength_microns = 0.55; // Green light (550nm)
    let aperture_diameter_microns = 100.0; // 100μm aperture

    let airy_disk = AiryDisk::new(wavelength_microns, aperture_diameter_microns);

    println!("Parameters:");
    println!(
        "  Wavelength: {:.2} μm ({:.0} nm)",
        wavelength_microns,
        wavelength_microns * 1000.0
    );
    println!("  Aperture diameter: {:.1} μm", aperture_diameter_microns);
    println!(
        "  First zero location (r₀): {:.4} radians",
        airy_disk.first_zero
    );
    println!(
        "  Full-width-half-maximum (FWHM): {:.4} radians",
        airy_disk.fwhm
    );

    // Debug: Check intensity at FWHM/2
    let fwhm_half = airy_disk.fwhm / 2.0;
    let intensity_at_fwhm_half = airy_disk.intensity(fwhm_half);
    println!(
        "  Debug: Intensity at FWHM/2 ({:.4}): {:.6}",
        fwhm_half, intensity_at_fwhm_half
    );
    println!();

    // Generate comparison samples with high resolution
    let num_points = 1000;
    let (radii, exact, gaussian, triangle) = airy_disk.generate_comparison_samples(num_points);

    // Calculate approximation errors
    let gaussian_mse = AiryDisk::calculate_mse(&exact, &gaussian);
    let triangle_mse = AiryDisk::calculate_mse(&exact, &triangle);

    // Calculate total summed absolute errors
    let gaussian_total_error: f64 = exact
        .iter()
        .zip(gaussian.iter())
        .map(|(e, g)| (e - g).abs())
        .sum();
    let triangle_total_error: f64 = exact
        .iter()
        .zip(triangle.iter())
        .map(|(e, t)| (e - t).abs())
        .sum();
    let approximations_total_error: f64 = gaussian
        .iter()
        .zip(triangle.iter())
        .map(|(g, t)| (g - t).abs())
        .sum();

    println!("Approximation Quality:");
    println!("  Gaussian MSE: {:.6}", gaussian_mse);
    println!("  Triangle MSE: {:.6}", triangle_mse);
    println!("  Gaussian Total Summed Error: {:.6}", gaussian_total_error);
    println!("  Triangle Total Summed Error: {:.6}", triangle_total_error);
    println!(
        "  Gaussian vs Triangle Total Summed Error: {:.6}",
        approximations_total_error
    );
    println!();

    // Find maximum errors for each approximation
    let (gauss_max_err, gauss_max_r, gauss_5pct_r) =
        AiryDisk::find_max_error(&radii, &exact, &gaussian);
    let (tri_max_err, tri_max_r, tri_5pct_r) = AiryDisk::find_max_error(&radii, &exact, &triangle);
    let (approx_max_err, approx_max_r, approx_5pct_r) =
        AiryDisk::find_max_error(&radii, &gaussian, &triangle);

    println!("Gaussian Approximation Error Analysis:");
    println!(
        "  Maximum error: {:.4} at r = {:.4}",
        gauss_max_err, gauss_max_r
    );
    if let Some(r) = gauss_5pct_r {
        println!(
            "  First 5% error at: r = {:.4} ({:.1}% of r₀)",
            r,
            100.0 * r / airy_disk.first_zero
        );
    }
    println!();

    println!("Triangle Approximation Error Analysis:");
    println!(
        "  Maximum error: {:.4} at r = {:.4}",
        tri_max_err, tri_max_r
    );
    if let Some(r) = tri_5pct_r {
        println!(
            "  First 5% error at: r = {:.4} ({:.1}% of r₀)",
            r,
            100.0 * r / airy_disk.first_zero
        );
    }
    println!();

    println!("Gaussian vs Triangle Approximation Difference:");
    println!(
        "  Maximum difference: {:.4} at r = {:.4}",
        approx_max_err, approx_max_r
    );
    if let Some(r) = approx_5pct_r {
        println!(
            "  First 5% difference at: r = {:.4} ({:.1}% of r₀)",
            r,
            100.0 * r / airy_disk.first_zero
        );
    }
    println!();

    // Display some key values at important points
    println!("Function Values at Key Points:");
    println!("  At r = 0 (center):");
    println!("    Exact: {:.6}", airy_disk.intensity(0.0));
    println!("    Gaussian: {:.6}", airy_disk.gaussian_approximation(0.0));
    println!("    Triangle: {:.6}", airy_disk.triangle_approximation(0.0));

    let r_half = airy_disk.first_zero * 0.5;
    println!("  At r = r₀/2 ({:.4}):", r_half);
    println!("    Exact: {:.6}", airy_disk.intensity(r_half));
    println!(
        "    Gaussian: {:.6}",
        airy_disk.gaussian_approximation(r_half)
    );
    println!(
        "    Triangle: {:.6}",
        airy_disk.triangle_approximation(r_half)
    );

    println!("  At r = r₀ ({:.4}) - first zero:", airy_disk.first_zero);
    println!(
        "    Exact: {:.6}",
        airy_disk.intensity(airy_disk.first_zero)
    );
    println!(
        "    Gaussian: {:.6}",
        airy_disk.gaussian_approximation(airy_disk.first_zero)
    );
    println!(
        "    Triangle: {:.6}",
        airy_disk.triangle_approximation(airy_disk.first_zero)
    );

    let r_1_5 = airy_disk.first_zero * 1.5;
    println!("  At r = 1.5×r₀ ({:.4}):", r_1_5);
    println!("    Exact: {:.6}", airy_disk.intensity(r_1_5));
    println!(
        "    Gaussian: {:.6}",
        airy_disk.gaussian_approximation(r_1_5)
    );
    println!(
        "    Triangle: {:.6}",
        airy_disk.triangle_approximation(r_1_5)
    );
    println!();

    // Create plots directory if it doesn't exist
    if !Path::new("plots").exists() {
        std::fs::create_dir("plots")?;
    }

    // Create a proper plot using plotters
    let plot_path = "plots/airy_disk_comparison.png";
    create_airy_comparison_plot(&radii, &exact, &gaussian, &triangle, &airy_disk, plot_path)?;

    println!("Plot saved to: {}", plot_path);
    println!();

    println!("Summary:");
    println!(
        "  • The first zero (dark ring) occurs at r₀ = {:.4} radians",
        airy_disk.first_zero
    );
    println!(
        "  • Gaussian approximation works best near the center (MSE: {:.6})",
        gaussian_mse
    );
    println!(
        "  • Triangle approximation is simpler but less accurate (MSE: {:.6})",
        triangle_mse
    );
    println!(
        "  • Total summed error: Gaussian = {:.3}, Triangle = {:.3}, Difference = {:.3}",
        gaussian_total_error, triangle_total_error, approximations_total_error
    );
    println!("  • Both approximations break down significantly beyond r₀");

    Ok(())
}

/// Create a detailed comparison plot of Airy disk and its approximations
fn create_airy_comparison_plot(
    radii: &[f64],
    exact: &[f64],
    gaussian: &[f64],
    triangle: &[f64],
    airy_disk: &AiryDisk,
    save_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Calculate normalized radii and error values
    let normalized_radii: Vec<f64> = radii.iter().map(|&r| r / airy_disk.first_zero).collect();
    let max_r_normalized = 2.0;

    let gaussian_error: Vec<f64> = exact
        .iter()
        .zip(gaussian.iter())
        .map(|(e, g)| e - g)
        .collect();

    let triangle_error: Vec<f64> = exact
        .iter()
        .zip(triangle.iter())
        .map(|(e, t)| e - t)
        .collect();

    // Set up the plot with higher resolution
    let root = BitMapBackend::new(save_path, (1600, 1200)).into_drawing_area();
    root.fill(&WHITE)?;
    let root = root.margin(20, 20, 20, 20);

    // Split into upper and lower panels
    let (upper, lower) = root.split_vertically(600);

    // Upper panel: Function comparison
    let mut upper_chart = ChartBuilder::on(&upper)
        .caption(
            "Airy Disk vs Approximations",
            ("sans-serif", 32).into_font().color(&BLACK),
        )
        .margin(15)
        .x_label_area_size(50)
        .y_label_area_size(80)
        .build_cartesian_2d(0.0..max_r_normalized, 0.0..1.1)?;

    upper_chart
        .configure_mesh()
        .x_desc("Normalized Radius (r/r₀)")
        .y_desc("Normalized Intensity")
        .axis_desc_style(("sans-serif", 20))
        .label_style(("sans-serif", 16))
        .draw()?;

    // Plot the exact Airy disk function
    upper_chart
        .draw_series(LineSeries::new(
            normalized_radii
                .iter()
                .zip(exact.iter())
                .map(|(&r, &i)| (r, i)),
            BLUE,
        ))?
        .label("Exact Airy Disk")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], BLUE.stroke_width(2)));

    // Plot the Gaussian approximation
    upper_chart
        .draw_series(LineSeries::new(
            normalized_radii
                .iter()
                .zip(gaussian.iter())
                .map(|(&r, &i)| (r, i)),
            RED,
        ))?
        .label("Gaussian Approximation")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED.stroke_width(2)));

    // Plot the Triangle approximation
    upper_chart
        .draw_series(LineSeries::new(
            normalized_radii
                .iter()
                .zip(triangle.iter())
                .map(|(&r, &i)| (r, i)),
            GREEN,
        ))?
        .label("Triangle Approximation")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREEN.stroke_width(2)));

    // Add vertical line at first zero (r₀)
    upper_chart.draw_series(std::iter::once(PathElement::new(
        vec![(1.0, 0.0), (1.0, 1.1)],
        BLACK,
    )))?;

    // Add FWHM annotation lines: y-axis to curve to x-axis
    let fwhm_half_normalized = airy_disk.fwhm / 2.0 / airy_disk.first_zero;
    let fwhm_intensity = 0.5; // Half maximum by definition

    // Horizontal line from y-axis to FWHM point
    upper_chart.draw_series(std::iter::once(PathElement::new(
        vec![
            (0.0, fwhm_intensity),
            (fwhm_half_normalized, fwhm_intensity),
        ],
        RGBColor(128, 128, 128).stroke_width(2),
    )))?;
    // Vertical line from FWHM point down to x-axis
    upper_chart.draw_series(std::iter::once(PathElement::new(
        vec![
            (fwhm_half_normalized, fwhm_intensity),
            (fwhm_half_normalized, 0.0),
        ],
        RGBColor(128, 128, 128).stroke_width(2),
    )))?;

    // Add text annotation for first zero
    upper_chart.draw_series(std::iter::once(Text::new(
        format!("r₀ = {:.3}", airy_disk.first_zero),
        (1.05, 1.0),
        ("sans-serif", 18).into_font().color(&BLACK),
    )))?;

    // Add text annotation for FWHM
    upper_chart.draw_series(std::iter::once(Text::new(
        "FWHM",
        (0.05, 0.55),
        ("sans-serif", 18)
            .into_font()
            .color(&RGBColor(128, 128, 128)),
    )))?;

    upper_chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.9))
        .border_style(BLACK)
        .label_font(("sans-serif", 18))
        .draw()?;

    // Lower panel: Error comparison
    let error_max = gaussian_error
        .iter()
        .chain(triangle_error.iter())
        .map(|e| e.abs())
        .fold(0.0, f64::max);
    let error_range = error_max * 1.1;

    let mut lower_chart = ChartBuilder::on(&lower)
        .caption(
            "Approximation Errors",
            ("sans-serif", 32).into_font().color(&BLACK),
        )
        .margin(15)
        .x_label_area_size(50)
        .y_label_area_size(80)
        .build_cartesian_2d(0.0..max_r_normalized, -error_range..error_range)?;

    lower_chart
        .configure_mesh()
        .x_desc("Normalized Radius (r/r₀)")
        .y_desc("Error (Exact - Approximation)")
        .axis_desc_style(("sans-serif", 20))
        .label_style(("sans-serif", 16))
        .draw()?;

    // Plot Gaussian error
    lower_chart
        .draw_series(LineSeries::new(
            normalized_radii
                .iter()
                .zip(gaussian_error.iter())
                .map(|(&r, &e)| (r, e)),
            RED,
        ))?
        .label("Gaussian Error")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], RED.stroke_width(2)));

    // Plot Triangle error
    lower_chart
        .draw_series(LineSeries::new(
            normalized_radii
                .iter()
                .zip(triangle_error.iter())
                .map(|(&r, &e)| (r, e)),
            GREEN,
        ))?
        .label("Triangle Error")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], GREEN.stroke_width(2)));

    // Add horizontal zero line
    lower_chart.draw_series(std::iter::once(PathElement::new(
        vec![(0.0, 0.0), (max_r_normalized, 0.0)],
        BLACK,
    )))?;

    // Add vertical line at first zero
    lower_chart.draw_series(std::iter::once(PathElement::new(
        vec![(1.0, -error_range), (1.0, error_range)],
        BLACK,
    )))?;

    lower_chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.9))
        .border_style(BLACK)
        .label_font(("sans-serif", 18))
        .draw()?;

    root.present()?;

    Ok(())
}
