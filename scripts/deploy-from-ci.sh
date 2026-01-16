#!/bin/bash
set -euo pipefail

# Deploy ARM64 binaries from CI artifacts to embedded devices
#
# This script downloads the latest ARM64 build artifacts from GitHub Actions
# and deploys them to the target device.
#
# Usage:
#   ./deploy-from-ci.sh --orin           # Deploy to Orin Nano
#   ./deploy-from-ci.sh --neut           # Deploy to Neutralino
#   ./deploy-from-ci.sh --orin --run     # Deploy and restart service
#
# Prerequisites:
#   - GitHub CLI (gh) installed and authenticated
#   - SSH access to target device

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Configuration
DEVICE_TYPE=""
REMOTE_HOST=""
REMOTE_BIN_DIR="rust-builds/meter-sim/target/release"
RESTART_SERVICE=false
ARTIFACT_NAME="arm64-binaries"
WORKFLOW_NAME="ARM64 Build"

# Host presets
ORIN_HOST="${ORIN_HOST:-meawoppl@orin-nano.tail944341.ts.net}"
NEUT_HOST="cosmicfrontiers@orin-005.tail944341.ts.net"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
print_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
print_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
print_error() { echo -e "${RED}[ERROR]${NC} $1"; }

usage() {
    echo "Usage: $0 [--orin|--neut] [OPTIONS]"
    echo ""
    echo "Deploy ARM64 binaries from CI artifacts to embedded devices."
    echo ""
    echo "Device Selection (one required):"
    echo "  --orin         Deploy to Orin Nano (${ORIN_HOST})"
    echo "  --neut         Deploy to Neutralino (${NEUT_HOST})"
    echo ""
    echo "Options:"
    echo "  --run          Restart fgs-server service after deploy"
    echo "  -h, --help     Show this help"
    echo ""
    echo "Prerequisites:"
    echo "  - GitHub CLI (gh) installed and authenticated"
    echo "  - Self-hosted runner must have built the artifacts"
    exit 0
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --orin)
            DEVICE_TYPE="orin"
            REMOTE_HOST="$ORIN_HOST"
            shift
            ;;
        --neut)
            DEVICE_TYPE="neut"
            REMOTE_HOST="$NEUT_HOST"
            shift
            ;;
        --run)
            RESTART_SERVICE=true
            shift
            ;;
        -h|--help)
            usage
            ;;
        *)
            print_error "Unknown option: $1"
            usage
            ;;
    esac
done

if [ -z "$DEVICE_TYPE" ]; then
    print_error "Device type required. Use --orin or --neut"
    usage
fi

# Check gh CLI
if ! command -v gh &> /dev/null; then
    print_error "GitHub CLI (gh) not found. Install from: https://cli.github.com/"
    exit 1
fi

# Check authentication
if ! gh auth status &> /dev/null; then
    print_error "Not authenticated with GitHub CLI. Run: gh auth login"
    exit 1
fi

print_info "Deploying to $DEVICE_TYPE ($REMOTE_HOST)"

# Step 1: Find latest successful workflow run
print_info "Finding latest successful ARM64 build..."
RUN_ID=$(gh run list \
    --workflow "$WORKFLOW_NAME" \
    --branch main \
    --status success \
    --limit 1 \
    --json databaseId \
    --jq '.[0].databaseId')

if [ -z "$RUN_ID" ] || [ "$RUN_ID" = "null" ]; then
    print_error "No successful ARM64 builds found"
    print_error "Make sure the self-hosted runner has completed a build"
    exit 1
fi

print_success "Found run ID: $RUN_ID"

# Step 2: Download artifacts
TEMP_DIR=$(mktemp -d)
print_info "Downloading artifacts to $TEMP_DIR..."

gh run download "$RUN_ID" \
    --name "$ARTIFACT_NAME" \
    --dir "$TEMP_DIR"

# Check what we got
if [ ! -f "$TEMP_DIR/fgs_server" ]; then
    print_error "fgs_server not found in artifacts"
    ls -la "$TEMP_DIR"
    rm -rf "$TEMP_DIR"
    exit 1
fi

print_success "Artifacts downloaded:"
ls -lh "$TEMP_DIR"

# Step 3: Verify binary architecture
print_info "Verifying binary architecture..."
ARCH_INFO=$(file "$TEMP_DIR/fgs_server")
if ! echo "$ARCH_INFO" | grep -q "ARM aarch64"; then
    print_error "Binary is not ARM64!"
    print_error "$ARCH_INFO"
    rm -rf "$TEMP_DIR"
    exit 1
fi
print_success "Binary is ARM64"

# Step 4: Copy to remote device
print_info "Copying binaries to $REMOTE_HOST..."

# Ensure remote directory exists
ssh "$REMOTE_HOST" "mkdir -p ~/$REMOTE_BIN_DIR"

# Copy binaries
scp "$TEMP_DIR/fgs_server" "$REMOTE_HOST:~/$REMOTE_BIN_DIR/"

if [ -f "$TEMP_DIR/calibration_controller" ]; then
    scp "$TEMP_DIR/calibration_controller" "$REMOTE_HOST:~/$REMOTE_BIN_DIR/"
fi

print_success "Binaries copied"

# Step 5: Cleanup temp dir
rm -rf "$TEMP_DIR"

# Step 6: Restart service if requested
if [ "$RESTART_SERVICE" = true ]; then
    print_info "Restarting fgs-server service..."
    ssh "$REMOTE_HOST" "sudo systemctl restart fgs-server.service" || {
        print_warning "Service restart failed (may not be installed)"
        print_info "Run deploy-fgs-*.sh --setup to install the service"
    }
fi

# Step 7: Verify
print_info "Verifying deployment..."
ssh "$REMOTE_HOST" "ls -lh ~/$REMOTE_BIN_DIR/fgs_server && file ~/$REMOTE_BIN_DIR/fgs_server"

print_success "============================================"
print_success "Deployment complete!"
print_success "============================================"
echo ""
print_info "Binary location: ~/$REMOTE_BIN_DIR/fgs_server"
if [ "$RESTART_SERVICE" = true ]; then
    print_info "Service restarted. Check status with:"
    print_info "  ssh $REMOTE_HOST 'sudo systemctl status fgs-server'"
else
    print_info "To restart the service:"
    print_info "  ssh $REMOTE_HOST 'sudo systemctl restart fgs-server'"
fi
