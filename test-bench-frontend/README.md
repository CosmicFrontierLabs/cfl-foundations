# Test Bench Frontends

Yew-based WebAssembly frontends for the meter-sim test bench servers.

## Overview

This package contains two web frontends:
- **Calibrate Frontend**: Interactive pattern selector for `calibrate_serve`
- **Camera Frontend**: Live camera monitoring for `cam_serve`

Both frontends feature a terminal-green aesthetic and real-time updates.

## Building

### Requirements

- Rust with `wasm32-unknown-unknown` target
- [Trunk](https://trunkrs.dev/) - WASM build tool

### Install Dependencies

```bash
# Install trunk (one-time setup)
cargo install --locked trunk

# Add WASM target (if not already installed)
rustup target add wasm32-unknown-unknown
```

### Build Both Frontends

```bash
# From the repository root
./scripts/build-yew-frontends.sh
```

This builds both frontends and outputs to:
- `test-bench-frontend/dist/calibrate/`
- `test-bench-frontend/dist/camera/`

### Build Individual Frontends

```bash
cd test-bench-frontend

# Calibrate frontend only
trunk build --release --config Trunk-calibrate.toml --filehash false

# Camera frontend only
trunk build --release --config Trunk-camera.toml --filehash false
```

### Development Mode

For faster rebuilds during development:

```bash
# Watch and rebuild on changes
trunk watch --config Trunk-calibrate.toml

# Or for camera frontend
trunk watch --config Trunk-camera.toml
```

## Output Structure

After building, the `dist/` directory contains:

```
dist/
├── calibrate/
│   ├── index.html
│   ├── calibrate_wasm.js
│   ├── calibrate_wasm_bg.wasm
│   └── shared-styles.css
└── camera/
    ├── index.html
    ├── camera_wasm.js
    ├── camera_wasm_bg.wasm
    └── shared-styles.css
```

The test bench servers (`calibrate_serve` and `cam_serve`) serve these files via the `/static/` endpoint.

## Troubleshooting

**Error: "trunk: command not found"**
```bash
cargo install --locked trunk
```

**Error: "wasm32-unknown-unknown target not installed"**
```bash
rustup target add wasm32-unknown-unknown
```

**Server can't find WASM files**
- Make sure you've run `./scripts/build-yew-frontends.sh`
- Check that `dist/` directories exist and contain `.wasm` files
- The servers look for files relative to the repository root

## Architecture

- **Language**: Rust compiled to WebAssembly
- **Framework**: Yew 0.21 (React-like component framework)
- **HTTP Client**: gloo-net (for API calls)
- **Styling**: Shared CSS with terminal-green theme

### API Endpoints Used

**Calibrate Frontend:**
- `GET /jpeg` - Current pattern as JPEG
- `GET /config` - Current pattern configuration
- `POST /config` - Update pattern configuration

**Camera Frontend:**
- `GET /jpeg` - Latest camera frame
- `GET /stats` - Camera statistics (FPS, temps, etc.)
- `GET /annotated` - Frame with AprilTag detection overlay

## Development

The frontend code is organized as:

```
src/
├── lib.rs              # Public exports
├── calibrate_app.rs    # Calibrate pattern UI component
├── calibrate_main.rs   # Calibrate WASM entry point
├── camera_app.rs       # Camera monitor UI component
└── camera_main.rs      # Camera WASM entry point
```

Both frontends refresh automatically:
- Images: Every 100ms
- Statistics: Every 1000ms
