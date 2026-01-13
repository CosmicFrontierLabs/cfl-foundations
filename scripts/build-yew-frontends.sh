#!/bin/bash
# Build script for Yew WASM frontends

set -e

echo "Building Yew frontends for test-bench..."

# Check if trunk is installed
if ! command -v trunk &> /dev/null; then
    echo "Error: trunk is not installed"
    echo "Install with: cargo install --locked trunk"
    exit 1
fi

# Check if wasm32-unknown-unknown target is installed
if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
    echo "Installing wasm32-unknown-unknown target..."
    rustup target add wasm32-unknown-unknown
fi

# Navigate to test-bench-frontend directory
cd "$(dirname "$0")/../test-bench-frontend"

# Build calibrate frontend
echo "Building calibrate frontend..."
trunk build --release --config Trunk-calibrate.toml --filehash false

# Build FGS frontend
echo "Building FGS frontend..."
trunk build --release --config Trunk-fgs.toml --filehash false

echo "Yew frontends built successfully!"
echo "Calibrate output: test-bench-frontend/dist/calibrate/"
echo "FGS output: test-bench-frontend/dist/fgs/"
