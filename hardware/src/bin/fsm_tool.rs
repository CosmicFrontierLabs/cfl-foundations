//! Unified CLI tool for PI E-727 fast steering mirror control.
//!
//! Combines functionality from multiple FSM binaries into a single tool with subcommands:
//! - `circle`: Trace circular patterns
//! - `steer`: Interactive steering with arrow keys
//! - `move`: Move to absolute/relative positions
//! - `resonance`: Excite and record resonance response
//! - `query`: Query axis positions and status
//! - `info`: Query device info and servo tuning parameters
//! - `off`: Disable servos

use std::f64::consts::PI;
use std::io::{self, Read, Write};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use hardware::pi::{GcsDevice, SpaParam, E727};
use strum::IntoEnumIterator;
use tracing::info;

/// Default E-727 IP address
const DEFAULT_IP: &str = "192.168.15.210";

/// Maximum fraction of range to use for circle (safety margin)
const MAX_RANGE_FRACTION: f64 = 0.80;

/// Tilt axis identifiers
const AXIS_X: &str = "1";
const AXIS_Y: &str = "2";

/// PI E-727 Fast Steering Mirror Control Tool
#[derive(Parser, Debug)]
#[command(name = "fsm_tool")]
#[command(about = "Unified control tool for PI E-727 fast steering mirror")]
#[command(version)]
struct Args {
    /// E-727 IP address
    #[arg(long, global = true, default_value = DEFAULT_IP)]
    ip: String,

    /// Force autozero even if already done
    #[arg(long, global = true)]
    force_atz: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Trace a circular pattern with the FSM
    Circle {
        /// Radius as percentage of working FOV (0-100)
        #[arg(short, long, default_value = "95")]
        radius_percent: f64,

        /// Period of one revolution in seconds
        #[arg(short, long, default_value = "1.0")]
        period: f64,

        /// Number of revolutions (0 = infinite)
        #[arg(short, long, default_value = "0")]
        count: u32,

        /// Maximum step size per update in µrad
        #[arg(long, default_value = "10")]
        step: f64,
    },

    /// Interactive FSM steering with arrow keys
    Steer,

    /// Move to a specified position
    Move {
        /// Axis to move (1, 2, 3, or 4)
        #[arg(short, long)]
        axis: Option<String>,

        /// Absolute position to move to (in physical units)
        #[arg(short, long)]
        position: Option<f64>,

        /// Relative distance to move
        #[arg(short, long)]
        relative: Option<f64>,

        /// Move to center of axis range
        #[arg(short, long)]
        center: bool,

        /// Maximum step size per command (safety limit)
        #[arg(long, default_value = "10")]
        max_step: f64,

        /// Timeout in seconds for motion to complete
        #[arg(short, long, default_value = "5")]
        timeout: u64,

        /// Don't wait for motion to complete
        #[arg(long)]
        no_wait: bool,
    },

    /// Excite and record FSM resonance response
    Resonance {
        /// Axis to test (1 or 2)
        #[arg(short, long, default_value = "1")]
        axis: String,

        /// Step size in µrad for excitation
        #[arg(short, long, default_value = "100")]
        step: f64,

        /// Sample rate divider (1 = fastest)
        #[arg(long, default_value = "1")]
        rate: u32,

        /// Output CSV file
        #[arg(short, long, default_value = "resonance.csv")]
        output: String,
    },

    /// Query current positions and status
    Query {
        /// Specific axis to query (queries all if not specified)
        #[arg(short, long)]
        axis: Option<String>,
    },

    /// Disable servos
    Off {
        /// Specific axis to disable (disables all if not specified)
        #[arg(short, long)]
        axis: Option<String>,
    },

    /// Query device info and SPA parameters
    Info {
        /// Dump all SPA (Set Parameter Access) values from the controller
        #[arg(long)]
        dump_params: bool,

        /// Show data recorder configuration
        #[arg(long)]
        recorder: bool,

        /// List all available parameters (HPA? command)
        #[arg(long)]
        hpa: bool,

        /// Show all info (equivalent to --dump-params --recorder)
        #[arg(long)]
        all: bool,
    },

    /// Interactive GCS command REPL
    Repl,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match args.command {
        Command::Circle {
            radius_percent,
            period,
            count,
            step,
        } => cmd_circle(
            &args.ip,
            args.force_atz,
            radius_percent,
            period,
            count,
            step,
        ),
        Command::Steer => cmd_steer(&args.ip, args.force_atz),
        Command::Move {
            axis,
            position,
            relative,
            center,
            max_step,
            timeout,
            no_wait,
        } => cmd_move(
            &args.ip,
            args.force_atz,
            axis,
            position,
            relative,
            center,
            max_step,
            timeout,
            no_wait,
        ),
        Command::Resonance {
            axis,
            step,
            rate,
            output,
        } => cmd_resonance(&args.ip, &axis, step, rate, &output),
        Command::Query { axis } => cmd_query(&args.ip, axis),
        Command::Off { axis } => cmd_off(&args.ip, axis),
        Command::Info {
            dump_params,
            recorder,
            hpa,
            all,
        } => cmd_info(&args.ip, dump_params || all, recorder || all, hpa),
        Command::Repl => cmd_repl(&args.ip),
    }
}

// ==================== Circle Command ====================

fn cmd_circle(
    ip: &str,
    force_atz: bool,
    radius_percent: f64,
    period: f64,
    count: u32,
    max_step: f64,
) -> Result<()> {
    info!("Connecting to E-727 at {}...", ip);
    let mut fsm = E727::connect_ip(ip)?;

    if fsm.autozero(force_atz)? {
        info!("Autozero completed");
    } else {
        info!("Autozero skipped (use --force-atz to re-run)");
    }

    let (min1, max1) = fsm.get_travel_range(AXIS_X)?;
    let (min2, max2) = fsm.get_travel_range(AXIS_Y)?;
    let unit = fsm.get_unit(AXIS_X)?;
    let (center1, center2) = fsm.get_xy_centers()?;

    info!(
        "Axis 1: center={:.1} {}, range=[{:.1}, {:.1}]",
        center1, unit, min1, max1
    );
    info!(
        "Axis 2: center={:.1} {}, range=[{:.1}, {:.1}]",
        center2, unit, min2, max2
    );

    // Calculate max radius (80% of available range for safety)
    let max_radius1 = (max1 - center1).min(center1 - min1) * MAX_RANGE_FRACTION;
    let max_radius2 = (max2 - center2).min(center2 - min2) * MAX_RANGE_FRACTION;
    let max_radius = max_radius1.min(max_radius2);

    let radius_pct = radius_percent.clamp(0.0, 100.0);
    let radius = max_radius * (radius_pct / 100.0);

    info!(
        "Circle: radius={:.1} {} ({:.0}% of {:.1} max), period={:.2}s, max_step={:.1}",
        radius, unit, radius_pct, max_radius, period, max_step
    );

    info!("Enabling servos...");
    fsm.set_servo(AXIS_X, true)?;
    fsm.set_servo(AXIS_Y, true)?;

    let mut cur1 = fsm.get_position(AXIS_X)?.clamp(min1, max1);
    let mut cur2 = fsm.get_position(AXIS_Y)?.clamp(min2, max2);
    info!("Starting from: ({:.1}, {:.1})", cur1, cur2);

    let angular_velocity = 2.0 * PI / period;
    info!("Starting circular motion (Ctrl+C to stop)...");

    let start_time = Instant::now();
    let mut revolutions = 0u32;
    let mut last_rev_check = 0.0f64;
    let mut last_report_time = Instant::now();
    let mut pointings_since_report = 0u64;

    loop {
        let elapsed = start_time.elapsed().as_secs_f64();
        let angle = elapsed * angular_velocity;

        let target1 = center1 + radius * angle.cos();
        let target2 = center2 + radius * angle.sin();

        fsm.move_xy_fast(target1, target2)?;
        pointings_since_report += 1;

        if last_report_time.elapsed() >= Duration::from_secs(1) {
            info!("{} pointings/sec", pointings_since_report);
            pointings_since_report = 0;
            last_report_time = Instant::now();
        }

        let current_rev = angle / (2.0 * PI);
        if current_rev.floor() > last_rev_check.floor() {
            revolutions += 1;
            info!("Revolution {} complete", revolutions);
            fsm.last_error()?;

            if count > 0 && revolutions >= count {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(1));
        last_rev_check = current_rev;
    }

    // Return to center
    info!("Returning to center...");
    for _ in 0..10000 {
        cur1 = step_toward_clamped(cur1, center1, max_step, min1, max1);
        cur2 = step_toward_clamped(cur2, center2, max_step, min2, max2);
        fsm.move_xy_fast(cur1, cur2)?;
        if (cur1 - center1).abs() < 0.1 && (cur2 - center2).abs() < 0.1 {
            break;
        }
    }

    info!("Disabling servos...");
    fsm.set_servo(AXIS_X, false)?;
    fsm.set_servo(AXIS_Y, false)?;

    info!("Done!");
    Ok(())
}

fn step_toward_clamped(current: f64, target: f64, max_step: f64, min: f64, max: f64) -> f64 {
    let clamped_target = target.clamp(min, max);
    let delta = clamped_target - current;
    let next = if delta.abs() <= max_step {
        clamped_target
    } else {
        current + delta.signum() * max_step
    };
    next.clamp(min, max)
}

// ==================== Steer Command ====================

fn cmd_steer(ip: &str, force_atz: bool) -> Result<()> {
    println!("Connecting to E-727 at {ip}...");
    let mut fsm = E727::connect_ip(ip)?;

    let (min_x, max_x) = fsm.get_travel_range(AXIS_X)?;
    let (min_y, max_y) = fsm.get_travel_range(AXIS_Y)?;
    let (center_x, center_y) = fsm.get_xy_centers()?;

    if fsm.autozero(force_atz)? {
        println!("Autozero completed");
    } else {
        println!("Autozero skipped (use --force-atz to re-run)");
    }

    let mut pos_x = fsm.get_position(AXIS_X)?;
    let mut pos_y = fsm.get_position(AXIS_Y)?;
    println!("Position: ({pos_x:.1}, {pos_y:.1})");

    fsm.set_servo(AXIS_X, true)?;
    fsm.set_servo(AXIS_Y, true)?;
    std::thread::sleep(Duration::from_millis(50));

    let step_sizes = [0.1, 1.0, 10.0, 100.0];
    let mut step_idx = 1;

    println!("\n=== FSM Steering ===");
    println!("Arrow keys: Move | S: Step size | C: Center | R: Record | Q: Quit\n");

    set_raw_mode()?;

    let result = (|| -> Result<()> {
        let mut stdin = io::stdin();
        let mut buf = [0u8; 3];

        loop {
            print!(
                "\rPos: ({:8.3}, {:8.3}) µrad | Step: {:5.1} µrad | ",
                pos_x, pos_y, step_sizes[step_idx]
            );
            io::stdout().flush()?;

            let n = stdin.read(&mut buf)?;
            if n == 0 {
                continue;
            }

            let mut dx = 0.0;
            let mut dy = 0.0;
            let step = step_sizes[step_idx];

            match &buf[..n] {
                [27, 91, 65] => dy = step,  // Up
                [27, 91, 66] => dy = -step, // Down
                [27, 91, 67] => dx = step,  // Right
                [27, 91, 68] => dx = -step, // Left
                [b'q'] | [b'Q'] | [27] => break,
                [b's'] | [b'S'] => {
                    step_idx = (step_idx + 1) % step_sizes.len();
                    continue;
                }
                [b'c'] | [b'C'] => {
                    pos_x = center_x;
                    pos_y = center_y;
                    fsm.move_to(AXIS_X, pos_x)?;
                    fsm.move_to(AXIS_Y, pos_y)?;
                    continue;
                }
                [b'r'] | [b'R'] => {
                    print!("\r Recording...                                        ");
                    io::stdout().flush()?;

                    let (errors, positions) =
                        fsm.record_position(AXIS_X, Duration::from_millis(200))?;

                    if let Some(filename) = write_capture_csv(&errors, &positions)? {
                        let n = errors.len().min(positions.len());
                        print!("\r Saved {filename} ({n} samples)                    ");
                        io::stdout().flush()?;
                    }
                    continue;
                }
                _ => continue,
            }

            if dx != 0.0 || dy != 0.0 {
                pos_x = (pos_x + dx).clamp(min_x, max_x);
                pos_y = (pos_y + dy).clamp(min_y, max_y);
                fsm.move_to(AXIS_X, pos_x)?;
                fsm.move_to(AXIS_Y, pos_y)?;
            }
        }
        Ok(())
    })();

    restore_terminal()?;
    println!("\n\nDisabling servos...");
    fsm.set_servo(AXIS_X, false)?;
    fsm.set_servo(AXIS_Y, false)?;
    println!("Done!");

    result
}

fn set_raw_mode() -> Result<()> {
    std::process::Command::new("stty")
        .arg("-echo")
        .arg("raw")
        .arg("-icanon")
        .status()?;
    Ok(())
}

fn restore_terminal() -> Result<()> {
    std::process::Command::new("stty")
        .arg("echo")
        .arg("cooked")
        .arg("icanon")
        .status()?;
    Ok(())
}

fn write_capture_csv(errors: &[f64], positions: &[f64]) -> Result<Option<String>> {
    if errors.is_empty() {
        return Ok(None);
    }

    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let filename = format!("capture_{ts}.csv");
    let mut file = std::fs::File::create(&filename)?;
    writeln!(file, "time_us,error_x,pos_x")?;

    let n = errors.len().min(positions.len());
    for i in 0..n {
        let time_us = i as f64 * 20.0; // 50kHz = 20µs
        writeln!(file, "{:.1},{:.6},{:.6}", time_us, errors[i], positions[i])?;
    }

    Ok(Some(filename))
}

// ==================== Move Command ====================

#[allow(clippy::too_many_arguments)]
fn cmd_move(
    ip: &str,
    force_atz: bool,
    axis: Option<String>,
    position: Option<f64>,
    relative: Option<f64>,
    center: bool,
    max_step: f64,
    timeout: u64,
    no_wait: bool,
) -> Result<()> {
    if position.is_none() && relative.is_none() && !center {
        bail!("Must specify --position, --relative, or --center");
    }

    info!("Connecting to E-727 at {}...", ip);
    let mut fsm = E727::connect_ip(ip)?;

    if fsm.autozero(force_atz)? {
        info!("Autozero completed");
    } else {
        info!("Autozero skipped (use --force-atz to re-run)");
    }

    let axis = axis.ok_or_else(|| anyhow::anyhow!("--axis is required for move operations"))?;

    let (min, max) = fsm.get_travel_range(&axis)?;
    let unit = fsm.get_unit(&axis)?;
    let current_pos = fsm.get_position(&axis)?;
    let servo_on = fsm.get_servo(&axis)?;

    info!(
        "Axis {}: current={:.3} {}, range=[{:.3}, {:.3}], servo={}",
        axis, current_pos, unit, min, max, servo_on
    );

    let mut target = if let Some(pos) = position {
        pos
    } else if let Some(rel) = relative {
        current_pos + rel
    } else {
        fsm.get_center(&axis)?
    };

    if target < min || target > max {
        bail!("Target position {target:.3} is outside range [{min:.3}, {max:.3}]");
    }

    let step = target - current_pos;
    if step.abs() > max_step {
        let clamped_step = step.signum() * max_step;
        let original_target = target;
        target = current_pos + clamped_step;
        info!(
            "Step size {:.3} exceeds max {:.1}, clamping to {:.3} (requested {:.3})",
            step.abs(),
            max_step,
            target,
            original_target
        );
    }

    info!("Target position: {:.3} {}", target, unit);

    if !servo_on {
        info!("Enabling servo on axis {}...", axis);
        fsm.set_servo(&axis, true)?;
    }

    info!("Moving axis {} to {:.3} {}...", axis, target, unit);
    fsm.move_to(&axis, target)?;

    if !no_wait {
        let timeout_dur = Duration::from_secs(timeout);
        info!("Waiting for motion to complete (timeout: {}s)...", timeout);

        let start = Instant::now();
        loop {
            if start.elapsed() > timeout_dur {
                bail!("Timeout waiting for motion to complete");
            }

            if fsm.is_on_target(&axis)? {
                break;
            }

            std::thread::sleep(Duration::from_millis(10));
        }

        let final_pos = fsm.get_position(&axis)?;
        let error = final_pos - target;
        info!(
            "Motion complete! Final position: {:.3} {} (error: {:.3} {})",
            final_pos, unit, error, unit
        );
    } else {
        info!("Motion command sent (not waiting for completion)");
    }

    Ok(())
}

// ==================== Resonance Command ====================

fn cmd_resonance(ip: &str, axis: &str, step: f64, rate: u32, output: &str) -> Result<()> {
    const RECORD_POSITION_ERROR: u8 = 3;
    const RECORD_CURRENT_POSITION: u8 = 2;
    const TRIGGER_IMMEDIATE: u8 = 4;

    info!("Connecting to E-727 at {}...", ip);
    let mut gcs = GcsDevice::connect_default_port(ip)?;

    // Query axis range
    let response = gcs.query(&format!("TMN? {axis}"))?;
    let min: f64 = response.split('=').nth(1).unwrap().trim().parse()?;
    let response = gcs.query(&format!("TMX? {axis}"))?;
    let max: f64 = response.split('=').nth(1).unwrap().trim().parse()?;
    let center = (min + max) / 2.0;
    info!(
        "Axis {} range: [{:.1}, {:.1}], center: {:.1}",
        axis, min, max, center
    );

    let response = gcs.query(&format!("POS? {axis}"))?;
    let current: f64 = response.split('=').nth(1).unwrap().trim().parse()?;
    info!("Current position: {:.3}", current);

    info!("Configuring data recorder...");
    gcs.send(&format!("RTR {rate}"))?;
    info!("Sample rate divider: {}", rate);

    gcs.send(&format!("DRC 1 {axis} {RECORD_POSITION_ERROR}"))?;
    gcs.send(&format!("DRC 2 {axis} {RECORD_CURRENT_POSITION}"))?;

    gcs.send(&format!("DRT 1 {TRIGGER_IMMEDIATE} 0"))?;
    gcs.send(&format!("DRT 2 {TRIGGER_IMMEDIATE} 0"))?;

    let response = gcs.query("DRC? 1")?;
    info!("Table 1 config: {}", response.trim());
    let response = gcs.query("DRC? 2")?;
    info!("Table 2 config: {}", response.trim());

    info!("Enabling servo on axis {}...", axis);
    gcs.send(&format!("SVO {axis} 1"))?;
    std::thread::sleep(Duration::from_millis(100));

    let start_pos = center - step / 2.0;
    let end_pos = center + step / 2.0;

    info!("Moving to start position {:.1}...", start_pos);
    gcs.send(&format!("MOV {axis} {start_pos}"))?;
    std::thread::sleep(Duration::from_millis(500));

    info!("Starting data recording...");
    info!(
        "Applying step: {:.1} -> {:.1} µrad ({:.1} µrad step)",
        start_pos, end_pos, step
    );
    gcs.send(&format!("MOV {axis} {end_pos}"))?;

    info!("Waiting for transient response...");
    std::thread::sleep(Duration::from_millis(200));

    info!("Stopping recording...");
    gcs.send("DRT 1 0 0")?;
    gcs.send("DRT 2 0 0")?;

    let response = gcs.query("DRL? 1")?;
    let num_points: usize = response
        .split('=')
        .nth(1)
        .unwrap_or("0")
        .trim()
        .parse()
        .unwrap_or(0);
    info!("Recorded {} points in table 1", num_points);

    if num_points == 0 {
        info!("No data recorded!");
        return Ok(());
    }

    info!("Reading recorded data...");
    let response = gcs.query(&format!("DRR? 1 {num_points} 1"))?;
    let error_data: Vec<f64> = response
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                None
            } else {
                trimmed.parse().ok()
            }
        })
        .collect();

    let response = gcs.query(&format!("DRR? 1 {num_points} 2"))?;
    let position_data: Vec<f64> = response
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                None
            } else {
                trimmed.parse().ok()
            }
        })
        .collect();

    info!(
        "Got {} error samples and {} position samples",
        error_data.len(),
        position_data.len()
    );

    let base_rate_hz = 50000.0;
    let sample_period_us = (rate as f64) / base_rate_hz * 1e6;

    info!("Writing to {}...", output);
    let mut file = std::fs::File::create(output)?;
    writeln!(file, "time_us,position_error,position")?;

    let n_samples = error_data.len().min(position_data.len());
    for i in 0..n_samples {
        let time_us = i as f64 * sample_period_us;
        writeln!(
            file,
            "{:.3},{:.6},{:.6}",
            time_us, error_data[i], position_data[i]
        )?;
    }

    info!("Done! {} samples written", n_samples);

    if !error_data.is_empty() {
        let max_error = error_data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_error = error_data.iter().cloned().fold(f64::INFINITY, f64::min);
        let peak_to_peak = max_error - min_error;
        info!(
            "Position error: min={:.3}, max={:.3}, pk-pk={:.3}",
            min_error, max_error, peak_to_peak
        );
    }

    info!("Disabling servo...");
    gcs.send(&format!("SVO {axis} 0"))?;

    info!("Done!");
    Ok(())
}

// ==================== Query Command ====================

fn cmd_query(ip: &str, axis: Option<String>) -> Result<()> {
    info!("Connecting to E-727 at {}...", ip);
    let mut fsm = E727::connect_ip(ip)?;

    let axes: Vec<String> = if let Some(ax) = axis {
        vec![ax]
    } else {
        fsm.connected_axes()?
    };

    for axis in &axes {
        let (min, max) = fsm.get_travel_range(axis)?;
        let unit = fsm.get_unit(axis)?;
        let current_pos = fsm.get_position(axis)?;
        let servo_on = fsm.get_servo(axis)?;
        let on_target = fsm.is_on_target(axis)?;
        let autozeroed = fsm.is_autozeroed(axis)?;

        info!(
            "Axis {}: position={:.3} {}, range=[{:.3}, {:.3}], servo={}, on_target={}, atz={}",
            axis, current_pos, unit, min, max, servo_on, on_target, autozeroed
        );
    }

    Ok(())
}

// ==================== Off Command ====================

fn cmd_off(ip: &str, axis: Option<String>) -> Result<()> {
    info!("Connecting to E-727 at {}...", ip);
    let mut fsm = E727::connect_ip(ip)?;

    let axes: Vec<String> = if let Some(ax) = axis {
        vec![ax]
    } else {
        fsm.connected_axes()?
    };

    for axis in &axes {
        info!("Disabling servo on axis {}...", axis);
        fsm.set_servo(axis, false)?;
    }

    info!("Servos disabled");
    Ok(())
}

// ==================== Info Command ====================

fn cmd_info(ip: &str, dump_params: bool, show_recorder: bool, show_hpa: bool) -> Result<()> {
    info!("Connecting to E-727 at {}...", ip);
    let mut gcs = GcsDevice::connect_default_port(ip)?;

    // Basic device info
    info!("=== Device Info ===");
    let response = gcs.query("*IDN?")?;
    info!("IDN: {}", response.trim());

    let response = gcs.query("SAI?")?;
    let axes: Vec<&str> = response
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    info!("Axes: {:?}", axes);

    let response = gcs.query("POS?")?;
    info!("Positions: {}", response.trim());

    let response = gcs.query("SVO?")?;
    info!("Servo states: {}", response.trim());

    // List all available parameters
    if show_hpa {
        info!("=== Available Parameters (HPA?) ===");
        let response = gcs.query("HPA?")?;
        // Print each parameter on its own line
        for line in response.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                info!("{}", trimmed);
            }
        }
    }

    // Data recorder info
    if show_recorder {
        info!("=== Data Recorder ===");
        let response = gcs.query("HDR?")?;
        info!("HDR? (available options):\n{}", response);

        let response = gcs.query("DRC?")?;
        info!("DRC? (current config): {}", response.trim());

        let response = gcs.query("DRT?")?;
        info!("DRT? (trigger): {}", response.trim());

        let response = gcs.query("RTR?")?;
        info!("RTR? (sample rate): {}", response.trim());

        let response = gcs.query("TNR?")?;
        info!("TNR? (num tables): {}", response.trim());
    }

    // All SPA parameters
    if dump_params {
        info!("=== SPA Parameters ===");

        // Calculate max name width for alignment
        let max_name_len = SpaParam::iter()
            .map(|p| p.to_string().len())
            .max()
            .unwrap_or(0);

        // First, query system-wide parameters (MaxItem=1)
        // Note: GCS requires axis identifier even for system-wide params, use axis 1
        info!("  System-wide:");
        for param in SpaParam::iter().filter(|p| p.is_system_wide()) {
            let response = gcs.query(&format!("SPA? 1 0x{:08X}", param.address()))?;
            // Parse value from response (format: "1=value")
            let value = response
                .split('=')
                .nth(1)
                .map(|s| s.trim())
                .unwrap_or(response.trim());
            info!("    {:width$} : {}", param, value, width = max_name_len);
        }

        // Then query per-axis parameters (MaxItem=4) for axes 1 and 2
        for axis in ["1", "2"] {
            info!("  Axis {}:", axis);
            for param in SpaParam::iter().filter(|p| p.max_items() == 4) {
                match gcs.query(&format!("SPA? {axis} 0x{:08X}", param.address())) {
                    Ok(response) => {
                        let value = response
                            .split('=')
                            .nth(1)
                            .map(|s| s.trim())
                            .unwrap_or(response.trim());
                        info!("    {:width$} : {}", param, value, width = max_name_len);
                    }
                    Err(e) => {
                        info!(
                            "    {:width$} : <error: {}>",
                            param,
                            e,
                            width = max_name_len
                        );
                    }
                }
            }
        }

        // Query other multi-item parameters (MaxItem != 1 and != 4)
        // These are per-input-channel (1-7) or per-trigger (1-3) parameters
        let other_params: Vec<_> = SpaParam::iter()
            .filter(|p| !p.is_system_wide() && p.max_items() != 4)
            .collect();

        if !other_params.is_empty() {
            info!("  Other (per-channel/table):");
            for param in other_params {
                let n_items = param.max_items();
                // Query all items for this parameter
                for item in 1..=n_items {
                    match gcs.query(&format!("SPA? {item} 0x{:08X}", param.address())) {
                        Ok(response) => {
                            let value = response
                                .split('=')
                                .nth(1)
                                .map(|s| s.trim())
                                .unwrap_or(response.trim());
                            if item == 1 {
                                info!("    {:width$} : {}", param, value, width = max_name_len);
                            } else {
                                info!("    {:width$} : {}", "", value, width = max_name_len);
                            }
                        }
                        Err(e) => {
                            if item == 1 {
                                info!(
                                    "    {:width$} : <error: {}>",
                                    param,
                                    e,
                                    width = max_name_len
                                );
                            } else {
                                info!("    {:width$} : <error: {}>", "", e, width = max_name_len);
                            }
                        }
                    }
                }
            }
        }
    }

    info!("Done!");
    Ok(())
}

// ==================== REPL Command ====================

fn cmd_repl(ip: &str) -> Result<()> {
    println!("Connecting to E-727 at {ip}...");
    let mut gcs = GcsDevice::connect_default_port(ip)?;

    let response = gcs.query("*IDN?")?;
    println!("Connected: {}", response.trim());
    println!();
    println!("GCS REPL - Enter commands (queries end with '?'), 'quit' to exit");
    println!("Examples: *IDN?, POS?, SVO 1 1, MOV 1 1000");
    println!();

    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        if stdin.read_line(&mut input)? == 0 {
            break; // EOF
        }

        let cmd = input.trim();
        if cmd.is_empty() {
            continue;
        }

        if cmd.eq_ignore_ascii_case("quit") || cmd.eq_ignore_ascii_case("exit") {
            println!("Bye!");
            break;
        }

        // If command ends with ?, it's a query - expect response
        if cmd.ends_with('?') {
            match gcs.query(cmd) {
                Ok(response) => {
                    for line in response.lines() {
                        println!("{line}");
                    }
                }
                Err(e) => println!("Error: {e}"),
            }
        } else {
            // It's a command - send and check for errors
            match gcs.send(cmd) {
                Ok(()) => println!("OK"),
                Err(e) => println!("Error: {e}"),
            }
        }
    }

    Ok(())
}
