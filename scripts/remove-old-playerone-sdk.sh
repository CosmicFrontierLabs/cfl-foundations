#!/bin/bash
set -euo pipefail

# Remove old PlayerOne SDK v2 that conflicts with v3.9.0

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
print_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
print_warning() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
print_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Check if running with sudo
if [ "$EUID" -ne 0 ]; then
    print_error "This script must be run with sudo"
    exit 1
fi

print_info "Removing old PlayerOne SDK v2 from /lib/aarch64-linux-gnu/"

# Check what's there
print_info "Current v2 libraries:"
ls -lah /lib/aarch64-linux-gnu/libPlayerOneCamera* 2>/dev/null || {
    print_success "No old v2 libraries found"
    exit 0
}

# Remove v2 SDK files
print_warning "Removing v2 SDK files..."
rm -fv /lib/aarch64-linux-gnu/libPlayerOneCamera.so
rm -fv /lib/aarch64-linux-gnu/libPlayerOneCamera.so.2
rm -fv /lib/aarch64-linux-gnu/libPlayerOneCamera.so.2.0.5

# Update library cache
print_info "Updating library cache..."
ldconfig

# Verify removal
print_info "Verifying removal..."
if ls /lib/aarch64-linux-gnu/libPlayerOneCamera* 2>/dev/null; then
    print_error "Some PlayerOne libraries still present!"
    exit 1
else
    print_success "Old v2 SDK successfully removed"
fi

print_success "Now rebuild binaries to link against v3.9.0 in /usr/local/lib/"
