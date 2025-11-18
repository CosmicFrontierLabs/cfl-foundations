# Yew Frontend Migration Guide

Oye beratna! This document explains how the meter-sim test-bench frontends were migrated from plain HTML/JavaScript to Yew-based WebAssembly applications.

## Overview

The `calibrate_serve` and `cam_serve` servers originally used inline HTML templates with embedded JavaScript. This migration converts them to modern Yew (Rust WebAssembly) frontends while maintaining:

1. **Same visual aesthetic** - Terminal-style green theme
2. **Same functionality** - All controls and displays work identically
3. **Same API** - Backend endpoints unchanged
4. **Type safety** - Rust compiler checks frontend code
5. **Shared styles** - CSS extracted to `shared-styles.css`

## Architecture

### Before (HTML/JS)

```
calibrate_serve.rs
  └─> templates/calibrate_view.html (embedded with rust-embed)
       ├─> Inline CSS
       └─> Inline JavaScript

cam_serve.rs -> camera_server.rs
  └─> templates/live_view.html (embedded with rust-embed)
       ├─> Inline CSS
       └─> Inline JavaScript
```

### After (Yew WASM)

```
calibrate_serve.rs
  ├─> Serves WASM bundle via /static/
  └─> API endpoints (/jpeg, /config, etc.)

frontend/
  ├─> shared-styles.css (extracted common styles)
  ├─> calibrate_app.rs (Yew component)
  ├─> calibrate_main.rs (WASM entry point)
  └─> dist/calibrate/ (trunk build output)
       ├─> calibrate_wasm.js
       ├─> calibrate_wasm_bg.wasm
       └─> index.html

camera_server.rs
  ├─> Serves WASM bundle via /static/
  └─> API endpoints (/jpeg, /raw, /stats, /zoom, etc.)

frontend/
  ├─> camera_app.rs (Yew component)
  ├─> camera_main.rs (WASM entry point)
  └─> dist/camera/ (trunk build output)
       ├─> camera_wasm.js
       ├─> camera_wasm_bg.wasm
       └─> index.html
```

## Dependency Management

### Problem: WASM vs Native Dependencies

Tokio, mio, and many other async I/O crates don't support `wasm32-unknown-unknown`. The solution is conditional compilation.

### Solution: Feature Flags

```toml
[features]
default = ["server"]

# Server-side dependencies (tokio, axum, camera drivers, etc.)
server = [
    "axum", "tokio", "tower-http", ...
]

# Frontend WASM dependencies (yew, gloo, wasm-bindgen)
yew-frontend = [
    "yew", "wasm-bindgen", "gloo-net", ...
]
```

All dependencies are optional and enabled via features:

```rust
// Cargo.toml
[dependencies]
axum = { version = "0.7", optional = true }
tokio = { version = "1.0", features = ["full"], optional = true }
yew = { version = "0.21", features = ["csr"], optional = true }
```

### Binary Requirements

```toml
# Server binaries need "server" feature
[[bin]]
name = "calibrate_serve"
required-features = ["server"]

# WASM binaries need "yew-frontend" feature  
[[bin]]
name = "calibrate_wasm"
required-features = ["yew-frontend"]
```

### Module Gating

```rust
// lib.rs
#[cfg(feature = "server")]
pub mod camera_server;

#[cfg(feature = "yew-frontend")]
pub mod frontend;
```

## Shared Styles

The `shared-styles.css` file contains all common styling:

- **Color scheme**: Green (#00ff00) on dark (#0a0a0a, #111)
- **Layout**: Three-column (25%-50%-25%)
- **Typography**: Courier New monospace
- **Components**: Buttons, inputs, labels, info panels
- **Canvas**: Histogram, zoom visualization

Both frontends use identical styling, ensuring visual consistency.

## Building

### Prerequisites

```bash
# Install trunk (WASM bundler)
cargo install --locked trunk

# Add WASM target
rustup target add wasm32-unknown-unknown
```

### Build Frontends

```bash
# From repository root
./scripts/build-yew-frontends.sh

# Or individually
cd test-bench/frontend
trunk build --release --config Trunk-calibrate.toml calibrate_index.html
trunk build --release --config Trunk-camera.toml camera_index.html
```

### Run Servers

```bash
# Build frontends first (one time or when frontend changes)
./scripts/build-yew-frontends.sh

# Run servers
cargo run --bin calibrate_serve
cargo run --bin cam_serve -- --sim
```

## Development Workflow

### Frontend Development (Hot Reload)

```bash
cd test-bench/frontend

# Calibrate frontend on port 8081
trunk serve --config Trunk-calibrate.toml calibrate_index.html

# Camera frontend on port 8082  
trunk serve --config Trunk-camera.toml camera_index.html
```

Note: `trunk serve` provides its own dev server. For full integration testing, build with `trunk build` and run the actual backend servers.

### Backend Development

```bash
# No changes needed - backends still compile normally
cargo build --bin calibrate_serve
cargo build --bin cam_serve
```

## Migration Benefits

1. **Type Safety**: Frontend code checked by Rust compiler
2. **Code Reuse**: Share types between frontend and backend
3. **Better Tooling**: Rust-analyzer, clippy, formatting
4. **Less JavaScript**: No more mixing JS with Rust codebase
5. **Component Model**: Yew provides React-like components
6. **Performance**: WASM can be faster than JS
7. **Maintainability**: Shared styles, no duplication

## Backward Compatibility

### Original Templates Preserved

The original HTML templates remain in `templates/`:
- `calibrate_view.html`
- `live_view.html`

These can be restored by reverting the server changes if needed.

### API Unchanged

All REST endpoints maintain their original contracts:
- `/jpeg` - Image endpoints
- `/config` - Configuration endpoints
- `/stats` - Statistics endpoints
- `/zoom` - Zoom endpoints

Frontend communicates with backend via HTTP - same as before.

## Troubleshooting

### WASM Won't Compile

```bash
# Make sure you're using --no-default-features
cargo check --target wasm32-unknown-unknown \
    --no-default-features \
    --features yew-frontend \
    --bin calibrate_wasm
```

### Trunk Not Found

```bash
cargo install --locked trunk
```

### Server Won't Find WASM Files

Make sure you've built the frontends:
```bash
./scripts/build-yew-frontends.sh
```

The servers expect WASM files at:
- `test-bench/frontend/dist/calibrate/`
- `test-bench/frontend/dist/camera/`

### Styles Not Applied

Check that `shared-styles.css` is in the dist directory. Trunk should copy it automatically via:
```html
<link data-trunk rel="css" href="/frontend/shared-styles.css" />
```

## Future Enhancements

Potential improvements for the Yew frontends:

1. **WebSocket Support**: Replace HTTP polling with WebSocket for real-time updates
2. **Offline Support**: Service workers for offline functionality
3. **Advanced Visualizations**: More sophisticated canvas rendering
4. **Mobile Responsive**: Adapt layout for smaller screens
5. **Accessibility**: ARIA labels, keyboard navigation
6. **Testing**: wasm-bindgen-test for frontend unit tests
7. **State Management**: Consider yewdux for complex state

## References

- [Yew Documentation](https://yew.rs/)
- [Trunk Documentation](https://trunkrs.dev/)
- [wasm-bindgen Book](https://rustwasm.github.io/wasm-bindgen/)
- [Web-sys Documentation](https://rustwasm.github.io/wasm-bindgen/web-sys/)

Taki, kopeng! The frontends are now fully Rust-ified. Sa-sa ke?
