# Test Bench Frontends

Yew-based WebAssembly frontends for the meter-sim test bench servers.

## Overview

This package contains two web frontends:
- **Calibrate Frontend**: Interactive pattern selector for `calibrate_serve`
- **FGS Frontend**: Fine guidance sensor monitoring for `fgs_server`

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
- `test-bench-frontend/dist/fgs/`

### Build Individual Frontends

```bash
cd test-bench-frontend

# Calibrate frontend only
trunk build --release --config Trunk-calibrate.toml --filehash false

# FGS frontend only
trunk build --release --config Trunk-fgs.toml --filehash false
```

### Development Mode

For faster rebuilds during development:

```bash
# Watch and rebuild on changes
trunk watch --config Trunk-calibrate.toml

# Or for FGS frontend
trunk watch --config Trunk-fgs.toml
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
└── fgs/
    ├── index.html
    ├── fgs_wasm.js
    ├── fgs_wasm_bg.wasm
    └── shared-styles.css
```

The test bench servers (`calibrate_serve` and `fgs_server`) serve these files via the `/static/` endpoint.

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

**FGS Frontend:**
- WebSocket stream for live status and image updates

## Development

The frontend code is organized as:

```
src/
├── lib.rs              # Public exports
├── calibrate_app.rs    # Calibrate pattern UI component
├── calibrate_main.rs   # Calibrate WASM entry point
├── fgs_app.rs          # FGS monitor UI component
├── fgs_main.rs         # FGS WASM entry point
├── fgs/                # FGS sub-components
└── ws_image_stream.rs  # Reusable WebSocket image viewer
```

Both frontends refresh automatically:
- Images: Every 100ms
- Statistics: Every 1000ms
