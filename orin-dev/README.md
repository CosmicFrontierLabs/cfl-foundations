# Orin Dev - Development Tools for Jetson Orin

Development and testing tools for the Jetson Orin platform. This package contains experimental and development software that is **not intended for flight**, unlike the `flight-software` package.

## Purpose

This package provides tools for:
- Testing and evaluating camera hardware (PlayerOne astronomy cameras, etc.)
- Prototyping new sensor integrations
- Development utilities and diagnostics
- Performance testing and benchmarking

## Binaries

### playerone_info

Enumerate and display properties of connected PlayerOne astronomy cameras.

```bash
# List all connected PlayerOne cameras
cargo run --bin playerone_info

# Show detailed properties
cargo run --bin playerone_info -- --detailed
```

## Building

### Local Build (x86_64)

```bash
cargo build --release --package orin-dev
```

### ARM64 Cross-Compilation (for Jetson Orin)

```bash
# Build all binaries
../scripts/build-arm64.sh orin-dev

# Build specific binary
../scripts/build-arm64.sh orin-dev playerone_info
```

## Deployment to Jetson Orin

### Deploy and Run

```bash
# Deploy and run playerone_info
../scripts/deploy-to-orin.sh --package orin-dev --binary playerone_info --run './playerone_info --detailed'

# Deploy all binaries and keep on remote
../scripts/deploy-to-orin.sh --package orin-dev --keep-remote
```

### Environment Variables

- `ORIN_HOST` - Remote Orin hostname/IP (default: cosmicfrontiers@192.168.15.229)

## Dependencies

### PlayerOne SDK

The PlayerOne SDK native libraries must be installed on the target system:

1. Download from [Player One Astronomy](https://player-one-astronomy.com/)
2. Install libraries to system path (e.g., `/usr/local/lib`)
3. Update `LD_LIBRARY_PATH` if needed:
   ```bash
   export LD_LIBRARY_PATH=/usr/local/lib:$LD_LIBRARY_PATH
   ```

On Jetson Orin:
```bash
# Example installation (adjust paths as needed)
sudo cp libPlayerOneCamera.so /usr/local/lib/
sudo ldconfig
```

## Non-Flight Software

**WARNING**: This package is for development and testing only. Code here should **not** be deployed to flight systems. For flight-qualified software, use the `flight-software` package.
