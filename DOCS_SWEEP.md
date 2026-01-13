# Documentation Sweep - Issue #527

Comprehensive analysis of documentation gaps across the codebase.

## CLI Binaries Missing `long_help`

High priority - these are user-facing interfaces:

| Binary | Args | long_help | Priority | Status |
|--------|------|-----------|----------|--------|
| `test-bench/src/bin/fgs_server.rs` | 7 | 7 | HIGH | DONE |
| `simulator/src/bin/sensor_shootout.rs` | 13 | 13 | HIGH | DONE |
| `monocle_harness/src/bin/fgs_shootout.rs` | 16 | 16 | HIGH | DONE |
| `monocle_harness/src/bin/tracking_demo.rs` | 12 | 12 | MEDIUM | DONE |
| `test-bench/src/bin/calibrate_serve.rs` | 8 | 0 | MEDIUM |
| `test-bench/src/bin/led_latency_test.rs` | 17 | 0 | MEDIUM |
| `hardware/src/bin/fsm_tool.rs` | 22 | 0 | MEDIUM |
| `simulator/src/bin/mag_to_flux.rs` | 10 | 0 | MEDIUM |
| `simulator/src/bin/zero_point_calculator.rs` | 9 | 0 | LOW |
| `simulator/src/bin/single_detection_matrix.rs` | 9 | 0 | LOW |
| `hardware/src/bin/mock_gyro.rs` | 9 | 0 | LOW |
| `simulator/src/bin/stellar_color_plot.rs` | 7 | 0 | LOW |
| `simulator/src/bin/detector_tuning.rs` | 7 | 0 | LOW |
| `simulator/src/bin/sensor-view-stats.rs` | 7 | 0 | LOW |
| `hardware/src/bin/listen_gyro.rs` | 6 | 0 | LOW |
| `simulator/src/bin/motion_simulator.rs` | 5 | 0 | LOW |
| `simulator/src/bin/dc_vs_z.rs` | 5 | 0 | LOW |
| `simulator/src/bin/detection_comparison.rs` | 4 | 0 | LOW |
| `test-bench/src/bin/playerone_info.rs` | 1 | 0 | LOW |
| `hardware/src/bin/print_roi_constraints.rs` | 1 | 0 | LOW |
| `hardware/src/bin/v4l2_stride_test.rs` | 1 | 0 | LOW |

**Good example to follow:** `test-bench/src/bin/dark_frame_analysis.rs` (8 args, 8 long_help)

## Public Structs Missing Doc Comments

### monocle/src/config.rs - DONE
- [x] `GuideStarFilters` - struct-level doc with example configuration
- [x] `FgsConfig` - struct-level doc explaining acquisition vs tracking phases
- [x] All fields documented with:
  - Valid ranges
  - Typical values
  - Interaction with other fields

### monocle/src/selection.rs
- [ ] `StarDetectionStats` - needs struct and field docs

### shared/src/tracking_message.rs
- [ ] `TrackingMessage` - has derive but no doc comment

## Public Functions Missing Doc Comments

### shared/src/camera_interface/mod.rs
- [ ] `Timestamp::new()` - needs args/returns docs
- [ ] `Timestamp::from_duration()` - needs fuller explanation
- [ ] `Timestamp::to_duration()` - only has "Convert to Duration", needs more
- [ ] `SensorConfig::new()` - needs arg explanations
- [ ] `SensorConfig::image_size()` - minimal doc
- [ ] `SensorConfig::width()` / `height()` - minimal docs
- [ ] `BitDepth::as_u8()` / `max_value()` / `from_u8()` - need docs
- [ ] `FrameData::new()` - needs docs
- [ ] `FrameData::get_saturation()` - needs docs

### test-bench/src/camera_server.rs
- [ ] `create_router()` - no docs at all

### test-bench/src/gpio.rs
- [ ] `detect_gpio_config()` - has partial Returns but needs more
- [ ] `GpioController::new()` - has partial docs
- [ ] `GpioController::new_from_line()` - has partial docs

### monocle_harness/src/helpers.rs
- [ ] Multiple helper functions with incomplete `# Returns` sections

### monocle_harness/src/tracking_plots.rs
- [ ] `TrackingPlotter::new()` - minimal doc
- [ ] `TrackingPlotter::with_config()` - minimal doc
- [ ] `TrackingPlotter::generate_plot()` - needs Returns/Examples

## Type Aliases Needing Enhanced Docs

### shared/src/units.rs
- [ ] `Wavelength` - has doc but could use usage examples
- [ ] Add `# Examples` showing:
  - Creating from nanometers
  - Converting between units
  - Common astronomical wavelengths

## Formatting Consistency Issues

### Coordinate Convention Documentation
Mixed usage of `(row, col)` vs `(x, y)` across codebase. Files to clarify:
- `shared/src/image_proc/overlay.rs` - has good docs, use as template
- `shared/src/image_proc/centroid.rs` - uses (row, col) internally
- `shared/src/image_proc/detection/aabb.rs` - documents (row, col)

**Recommendation:** Standardize on:
- Array indexing: `(row, col)` - row-major, matches ndarray
- Image coordinates: `(x, y)` - column-major, matches image libs
- Always document which convention is used

### Missing `# Returns` Sections
Many functions have doc comments but lack `# Returns`:
- `monocle_harness/src/helpers.rs` - uses `*` instead of `# Returns`
- `test-bench/src/display_utils.rs` - partial coverage
- `shared/src/frame_writer.rs` - missing

### Missing `# Examples` Sections
Would benefit from examples:
- `shared/src/units.rs` - Temperature/Length/Wavelength conversions
- `shared/src/camera_interface/mod.rs` - Timestamp operations
- `monocle/src/config.rs` - Common configuration patterns

## Suggested Order of Work

1. **High-priority CLI bins** (cam_track, sensor_shootout, fgs_shootout)
2. **Core config structs** (GuideStarFilters, FgsConfig)
3. **Public API functions** (camera_interface, tracking_message)
4. **Helper functions** (monocle_harness, test-bench utilities)
5. **Type aliases** (units.rs)
6. **Formatting cleanup** (coordinate conventions)
