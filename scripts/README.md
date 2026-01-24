# Development Scripts

This directory contains scripts for git hooks, building, and deploying to NSV device (orin-005).

## Build & Deployment Scripts

### build-remote.sh
Build on cfl-test-bench and deploy to target devices.

Usage:
```bash
# Build and deploy to NSV (orin-005)
./scripts/build-remote.sh --package test-bench --binary fgs_server --nsv

# Build on test-bench only (no deployment)
./scripts/build-remote.sh --package test-bench --binary fgs_server --test-bench

# Build and run
./scripts/build-remote.sh --package test-bench --binary fgs_server --nsv --run './fgs_server'
```

Options:
- `--package PKG` - Package to build (e.g., test-bench)
- `--binary BIN` - Binary to build
- `--nsv` - Deploy to NSV device (orin-005)
- `--test-bench` - Build on cfl-test-bench only
- `--features FEAT` - Cargo features to enable
- `--run CMD` - Command to run after deployment

Environment variables:
- `NSV_HOST` - Override NSV host (default: cosmicfrontier@orin-005.tail944341.ts.net)

### deploy-fgs.sh
Deploy fgs_server to NSV device with frontend files.

Usage:
```bash
# Update existing deployment
./scripts/deploy-fgs.sh

# Full setup (install systemd service)
./scripts/deploy-fgs.sh --setup
```

### build-arm64.sh
Cross-compile packages for ARM64.

Usage:
```bash
# Build entire package
./scripts/build-arm64.sh <package-name>

# Build specific binary
./scripts/build-arm64.sh <package-name> <binary-name>
```

## Git Hooks Scripts

### install-hooks.sh
Installs pre-commit hooks that match the CI pipeline checks:
- Runs `cargo fmt` to check code formatting
- Runs `cargo clippy` with warnings as errors

Usage:
```bash
./scripts/install-hooks.sh
```

### uninstall-hooks.sh
Removes the installed pre-commit hooks.

Usage:
```bash
./scripts/uninstall-hooks.sh
```

## CI Parity
The hooks are designed to match the checks performed in CI:
- Format checking: `cargo fmt --all -- --check`
- Clippy linting: `cargo clippy -- -D warnings`

This ensures that code passing local checks will also pass CI checks.

## Bypassing Hooks
If you need to commit without running the hooks (not recommended):
```bash
git commit --no-verify
```
