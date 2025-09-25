# Monocle - Modular Orientation, Navigation & Optical Control Logic Engine

## Tracking Demo Binary

The `tracking_demo` binary allows you to test and visualize the Fine Guidance System (FGS) tracking performance with various motion patterns.

### Running the Tracking Demo

Basic usage:
```bash
# Run with default sine wave motion in RA
cargo run --bin tracking_demo

# Run with different motion patterns
cargo run --bin tracking_demo --motion sine_dec
cargo run --bin tracking_demo --motion circular
cargo run --bin tracking_demo --motion drift
cargo run --bin tracking_demo --motion step
cargo run --bin tracking_demo --motion chaotic
```

### Available Options

```bash
cargo run --bin tracking_demo -- --help
```

Key parameters:
- `--motion <TYPE>`: Motion pattern type (sine_ra, sine_dec, circular, drift, step, chaotic)
- `-t, --duration <SECONDS>`: Simulation duration (default: 10.0)
- `--output <FILENAME>`: Output plot filename (default: tracking_{motion}.png)
- `--frame-rate <HZ>`: Frame rate in Hz (default: 10.0)
- `--acquisition-frames <N>`: Number of acquisition frames (default: 3)
- `--min-snr <VALUE>`: Minimum SNR for guide star selection (default: 10.0)
- `--max-guide-stars <N>`: Maximum number of guide stars (default: 3)
- `--roi-size <PIXELS>`: ROI size in pixels (default: 32)
- `--centroid-multiplier <VALUE>`: Centroid radius multiplier (times FWHM) (default: 5.0)
- `--width <PIXELS>`: Plot width (default: 2400)
- `--height <PIXELS>`: Plot height (default: 1600)
- `-v, --verbose`: Enable verbose output

### Example Test Cases

#### Test 1: Basic Tracking with Sine Wave Motion
```bash
cargo run --bin tracking_demo --motion sine_ra --duration 20
```
Tests basic tracking performance with sinusoidal motion in RA.

#### Test 2: Fast Frame Rate with Circular Motion
```bash
cargo run --bin tracking_demo --motion circular --frame-rate 30 --duration 15
```
Tests tracking at higher frame rates with circular motion pattern.

#### Test 3: Large Centroid Radius with Drift Motion
```bash
cargo run --bin tracking_demo --motion drift --centroid-multiplier 8.0
```
Tests tracking with expanded centroid calculation radius during drift.

#### Test 4: Minimal Configuration for Quick Tests
```bash
cargo run --bin tracking_demo --acquisition-frames 1 --max-guide-stars 2 --roi-size 16 --duration 5
```
Fast test with minimal processing overhead.

#### Test 5: Stress Test with Chaotic Motion
```bash
cargo run --bin tracking_demo --motion chaotic --duration 30 --frame-rate 20 --verbose
```
Tests tracking robustness under unpredictable motion patterns.

#### Test 6: High-Resolution Plot Output
```bash
cargo run --bin tracking_demo --width 3600 --height 2400 --output high_res_tracking.png
```
Generates high-resolution tracking visualization plot.

### Output

All plots are saved to the `plots/` directory. The plots show:
- **Blue regions**: FGS in tracking mode (locked on stars)
- **Red regions**: FGS in acquisition mode (searching for lock)
- **Green bands**: Transition moments when lock is established
- **Blue dots**: Actual position (ground truth)
- **Red dots**: Estimated position from FGS

The plot contains two subplots:
- **Top**: X position tracking over time
- **Bottom**: Y position tracking over time

### Interpreting Results

Good tracking is indicated by:
- Quick acquisition (short red region at start)
- Stable tracking (blue regions)
- Close alignment between actual (blue dots) and estimated (red dots) positions
- Smooth tracking without jumps or loss of lock