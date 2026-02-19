# cfl-foundations

Shared foundation crates for Cosmic Frontier Labs space telescope software.

## Crates

| Crate | Description |
|-------|-------------|
| **meter-math** | Mathematical algorithms: quaternions, ICP point cloud alignment, interpolation, matrix transforms, statistics |
| **shared** | Image processing, sensor modeling, star projection, frame writing, camera interface traits |
| **shared-wasm** | Lightweight WASM-compatible types for frontend/backend serialization |

## Usage

Add as a git dependency in your `Cargo.toml`:

```toml
[dependencies]
meter-math = { git = "https://github.com/CosmicFrontierLabs/cfl-foundations" }
shared = { git = "https://github.com/CosmicFrontierLabs/cfl-foundations" }
shared-wasm = { git = "https://github.com/CosmicFrontierLabs/cfl-foundations" }
```

## Building

```bash
cargo build
cargo test
```
