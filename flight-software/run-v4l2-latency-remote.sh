#!/bin/bash
set -euo pipefail

# Remote V4L2 latency test runner - builds, deploys, runs, and retrieves PNG
# Usage: ./run-v4l2-latency-remote.sh [OPTIONS]

# Configuration
REMOTE_HOST="cosmicfrontiers@192.168.15.229"
TARGET="aarch64-unknown-linux-gnu"
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Parse command line arguments
MODE="long"
FRAMES=10
OUTPUT_NAME=""
SKIP_BUILD=false
VIDEO_DEVICE="/dev/video0"
LOCAL_PNG_NAME=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --mode)
            MODE="$2"
            shift 2
            ;;
        --frames)
            FRAMES="$2"
            shift 2
            ;;
        --output)
            OUTPUT_NAME="$2"
            shift 2
            ;;
        --local-name)
            LOCAL_PNG_NAME="$2"
            shift 2
            ;;
        --skip-build)
            SKIP_BUILD=true
            shift
            ;;
        --device)
            VIDEO_DEVICE="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --mode MODE         Capture mode: short or long (default: long)"
            echo "  --frames N          Number of frames to capture (default: 10)"
            echo "  --output NAME       Output PNG filename on remote (default: timestamp)"
            echo "  --local-name NAME   Local filename for retrieved PNG (default: same as remote)"
            echo "  --skip-build        Skip the build step (use existing binary)"
            echo "  --device DEV        Video device path (default: /dev/video0)"
            echo "  -h, --help          Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0 --mode short --frames 5 --output test_frame"
            echo "  $0 --output capture_001 --local-name local_capture.png"
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

print_info "V4L2 Latency Remote Test Runner"
print_info "Remote host: $REMOTE_HOST"
print_info "Mode: $MODE, Frames: $FRAMES"

# Step 1: Build for aarch64 target
if [ "$SKIP_BUILD" = false ]; then
    print_info "Building v4l2_latency for $TARGET..."
    
    cd "$SCRIPT_DIR"
    
    # Ensure the target is available
    rustup target add $TARGET 2>/dev/null || true
    
    # Build the binary
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    cargo build --target $TARGET --release --bin v4l2_latency
    
    if [ $? -eq 0 ]; then
        print_success "Build completed successfully"
    else
        print_error "Build failed"
        exit 1
    fi
else
    print_warning "Skipping build step (using existing binary)"
fi

# Step 2: Copy binary to remote
print_info "Deploying v4l2_latency to remote host..."
scp "$PROJECT_ROOT/target/$TARGET/release/v4l2_latency" "$REMOTE_HOST:/home/cosmicfrontiers/"

if [ $? -eq 0 ]; then
    print_success "Binary deployed successfully"
else
    print_error "Failed to deploy binary"
    exit 1
fi

# Step 3: Run v4l2_latency on remote
print_info "Running v4l2_latency on remote host..."

# Build the command - need sudo for GPIO access
REMOTE_CMD="sudo ./v4l2_latency --device $VIDEO_DEVICE --mode $MODE --frames $FRAMES --save-last-frame"

if [ -n "$OUTPUT_NAME" ]; then
    REMOTE_CMD="$REMOTE_CMD --output-name $OUTPUT_NAME"
fi

# Run the command and capture output
print_info "Executing: $REMOTE_CMD"
SSH_OUTPUT=$(ssh $REMOTE_HOST "$REMOTE_CMD" 2>&1)

if [ $? -eq 0 ]; then
    print_success "v4l2_latency executed successfully"
    
    # Parse the output to find the saved filename
    SAVED_FILE=$(echo "$SSH_OUTPUT" | grep "Saved frame as" | sed 's/.*Saved frame as //')
    
    if [ -n "$SAVED_FILE" ]; then
        print_info "Remote saved file: $SAVED_FILE"
    else
        print_warning "Could not determine saved filename from output"
        # If output name was specified, assume it worked
        if [ -n "$OUTPUT_NAME" ]; then
            if [[ "$OUTPUT_NAME" == *.png ]]; then
                SAVED_FILE="$OUTPUT_NAME"
            else
                SAVED_FILE="${OUTPUT_NAME}.png"
            fi
        else
            print_error "Cannot determine remote filename"
            exit 1
        fi
    fi
else
    print_error "v4l2_latency execution failed"
    echo "$SSH_OUTPUT"
    exit 1
fi

# Step 4: Retrieve PNG file
print_info "Retrieving PNG from remote host..."

# Determine local filename
if [ -n "$LOCAL_PNG_NAME" ]; then
    LOCAL_FILE="$LOCAL_PNG_NAME"
else
    LOCAL_FILE="$SAVED_FILE"
fi

# Ensure local file has .png extension
if [[ ! "$LOCAL_FILE" == *.png ]]; then
    LOCAL_FILE="${LOCAL_FILE}.png"
fi

scp "$REMOTE_HOST:/home/cosmicfrontiers/$SAVED_FILE" "$SCRIPT_DIR/$LOCAL_FILE"

if [ $? -eq 0 ]; then
    print_success "PNG retrieved successfully: $LOCAL_FILE"
    
    # Get file info
    FILE_SIZE=$(ls -lh "$SCRIPT_DIR/$LOCAL_FILE" | awk '{print $5}')
    print_info "File size: $FILE_SIZE"
    
    # Try to get image dimensions if imagemagick is installed
    if command -v identify &> /dev/null; then
        DIMENSIONS=$(identify -format "%wx%h" "$SCRIPT_DIR/$LOCAL_FILE" 2>/dev/null)
        if [ -n "$DIMENSIONS" ]; then
            print_info "Image dimensions: $DIMENSIONS"
        fi
    fi
else
    print_error "Failed to retrieve PNG"
    exit 1
fi

# Step 5: Clean up remote PNG (optional) - need sudo since file was created with sudo
print_info "Cleaning up remote PNG..."
ssh $REMOTE_HOST "sudo rm -f /home/cosmicfrontiers/$SAVED_FILE"

# Final report
echo ""
echo "====================================="
print_success "V4L2 latency test completed!"
print_info "Local PNG saved as: $SCRIPT_DIR/$LOCAL_FILE"
echo "====================================="

# Display the latency measurement output
echo ""
echo "Latency measurements:"
echo "$SSH_OUTPUT" | grep -E "^[0-9]+ [0-9]+$" || true