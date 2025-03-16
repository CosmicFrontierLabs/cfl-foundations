//! Tool for filtering Gaia catalog files to keep only bright stars
//!
//! This utility filters Gaia catalog files to keep only stars brighter than
//! a specified magnitude threshold (default: 20.0) and saves a smaller file
//! containing only essential fields: source_id, ra, dec, and phot_g_mean_mag.
//!
//! Usage:
//!   cargo run --example gaia_filter -- [options]
//!
//! Options:
//!   --input PATH       Input Gaia catalog file (CSV or gzipped CSV)
//!   --output PATH      Output file path (.csv or .bin extension)
//!   --magnitude FLOAT  Maximum magnitude threshold (default: 20.0)
//!   --list             List cached Gaia files
//!   --binary           Save in binary format instead of CSV (faster loading)

use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use flate2::read::GzDecoder;
use starfield::catalogs::{BinaryCatalog, MinimalStar};
use starfield::data::list_cached_gaia_files;

/// Filter Gaia catalog data from input file to output file
fn filter_gaia_file<P: AsRef<Path>, Q: AsRef<Path>>(
    input_path: P,
    output_path: Q,
    magnitude_limit: f64,
    use_binary: bool,
) -> Result<usize, Box<dyn std::error::Error>> {
    println!(
        "Filtering Gaia catalog file: {}",
        input_path.as_ref().display()
    );

    // Check if input file exists
    let input_file = File::open(&input_path)?;

    // Determine if the file is gzipped or not
    let path_str = input_path.as_ref().to_string_lossy().to_string();
    let is_gzipped = path_str.ends_with(".gz");

    // Create appropriate reader
    let reader: Box<dyn BufRead> = if is_gzipped {
        println!("Detected gzipped file, decompressing...");
        let decoder = GzDecoder::new(input_file);
        Box::new(BufReader::new(decoder))
    } else {
        Box::new(BufReader::new(input_file))
    };

    // Process the input file
    let mut lines_iter = reader.lines();

    // Read header line to determine column positions
    let header = match lines_iter.next() {
        Some(Ok(line)) => line,
        _ => return Err("Failed to read header from Gaia file".into()),
    };

    // Parse header to find column indices
    let headers: Vec<&str> = header.split(',').collect();
    let find_column = |name: &str| -> Result<usize, Box<dyn std::error::Error>> {
        headers
            .iter()
            .position(|&h| h == name)
            .ok_or_else(|| format!("Missing column: {}", name).into())
    };

    // Find required column indices
    let source_id_idx = find_column("source_id")?;
    let ra_idx = find_column("ra")?;
    let dec_idx = find_column("dec")?;
    let g_mag_idx = find_column("phot_g_mean_mag")?;

    let mut processed_lines = 0;
    let mut kept_stars = 0;
    let mut progress_marker = 100000;

    // Create description for catalog
    let desc = format!("Gaia catalog filtered to magnitude {}", magnitude_limit);

    // Collect stars first, then create catalog
    let mut filtered_stars = Vec::new();

    // Process data lines
    for line_result in lines_iter {
        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Error reading line: {}", e);
                continue;
            }
        };

        processed_lines += 1;

        // Show progress
        if processed_lines >= progress_marker {
            println!(
                "Processed {} lines, kept {} stars",
                processed_lines, kept_stars
            );
            progress_marker += 100000;
        }

        if line.trim().is_empty() {
            continue;
        }

        // Split line into fields
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() < headers.len() {
            continue; // Skip lines with insufficient columns
        }

        // Parse the magnitude first to filter early
        let g_mag = match fields[g_mag_idx].parse::<f64>() {
            Ok(mag) => mag,
            Err(_) => continue,
        };

        // Skip stars fainter than magnitude limit
        if g_mag > magnitude_limit {
            continue;
        }

        // Parse required fields
        let source_id = match fields[source_id_idx].parse::<u64>() {
            Ok(id) => id,
            Err(_) => continue,
        };

        let ra = match fields[ra_idx].parse::<f64>() {
            Ok(ra) => ra,
            Err(_) => continue,
        };

        let dec = match fields[dec_idx].parse::<f64>() {
            Ok(dec) => dec,
            Err(_) => continue,
        };

        // Create star object and add to collection
        let star = MinimalStar::new(source_id, ra, dec, g_mag);
        filtered_stars.push(star);
        kept_stars += 1;
    }

    // Create catalog from collected stars
    let catalog = BinaryCatalog::from_stars(filtered_stars, &desc);

    // Save the catalog in either binary or CSV format
    if use_binary {
        // Save to binary format
        catalog.save(&output_path)?;
    } else {
        // Save to CSV format
        let output_file = File::create(&output_path)?;
        let mut writer = BufWriter::new(output_file);

        // Write CSV header
        writeln!(writer, "source_id,ra,dec,phot_g_mean_mag")?;

        // Write each star as CSV
        for star in catalog.stars() {
            writeln!(
                writer,
                "{},{},{},{}",
                star.id, star.ra, star.dec, star.magnitude
            )?;
        }
    }

    println!("Completed filtering:");
    println!("  Processed {} lines", processed_lines);
    println!(
        "  Kept {} stars with magnitude <= {}",
        kept_stars, magnitude_limit
    );
    println!("  Output written to: {}", output_path.as_ref().display());
    println!("  Format: {}", if use_binary { "Binary" } else { "CSV" });

    Ok(kept_stars)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // Default values
    let mut input_path = None;
    let mut output_path = None;
    let mut magnitude_limit = 20.0;
    let mut list_files = false;
    let mut use_binary = false;

    // Parse command-line arguments
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                if i + 1 < args.len() {
                    input_path = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return Err("Missing value for --input".into());
                }
            }
            "--output" => {
                if i + 1 < args.len() {
                    output_path = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    return Err("Missing value for --output".into());
                }
            }
            "--magnitude" => {
                if i + 1 < args.len() {
                    magnitude_limit = args[i + 1].parse()?;
                    i += 2;
                } else {
                    return Err("Missing value for --magnitude".into());
                }
            }
            "--list" => {
                list_files = true;
                i += 1;
            }
            "--binary" => {
                use_binary = true;
                i += 1;
            }
            _ => {
                println!("Unknown argument: {}", args[i]);
                i += 1;
            }
        }
    }

    println!("Gaia Catalog Filter Tool");
    println!("=======================");

    if list_files {
        println!("Listing cached Gaia catalog files:");
        let files = list_cached_gaia_files()?;

        if files.is_empty() {
            println!("No files found in cache.");
        } else {
            for (i, path) in files.iter().enumerate() {
                println!("  {}. {}", i + 1, path.display());
            }
            println!("Total: {} files", files.len());
        }
        return Ok(());
    }

    // Auto-detect binary format from file extension if not explicitly set
    if let Some(path) = &output_path {
        if path.ends_with(".bin") && !use_binary {
            println!("Detected .bin extension, using binary format");
            use_binary = true;
        }
    }

    // Check if input and output paths are provided
    if input_path.is_none() || output_path.is_none() {
        println!("Usage:");
        println!("  cargo run --example gaia_filter -- --input <input_file> --output <output_file> [--magnitude <limit>]");
        println!("  cargo run --example gaia_filter -- --list");
        println!("");
        println!("Options:");
        println!("  --input PATH       Input Gaia catalog file (CSV or gzipped CSV)");
        println!("  --output PATH      Output file path (.csv or .bin extension)");
        println!("  --magnitude FLOAT  Maximum magnitude threshold (default: 20.0)");
        println!("  --binary           Save in binary format instead of CSV (faster loading)");
        println!("  --list             List cached Gaia files");

        return Err("Missing required arguments".into());
    }

    // Filter the file
    filter_gaia_file(
        input_path.unwrap(),
        output_path.unwrap(),
        magnitude_limit,
        use_binary,
    )?;

    Ok(())
}
