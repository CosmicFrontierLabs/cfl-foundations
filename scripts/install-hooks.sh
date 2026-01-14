#!/bin/bash
# Install git hooks for this repository
#
# This script configures git to use hooks from .githooks/ directory.
# Hooks are versioned in the repo so changes take effect immediately.

set -e

REPO_ROOT="$(git rev-parse --show-toplevel)"
GITHOOKS_DIR="$REPO_ROOT/.githooks"

echo "Configuring git to use .githooks/ directory..."

# Set git to use .githooks as the hooks directory
git config core.hooksPath .githooks

echo "✅ Git hooks configured successfully!"
echo ""
echo "Hooks in .githooks/ will now run automatically:"
echo "  • pre-commit: Format check, cargo check, clippy, doctest check"
echo "  • commit-msg: Reject commits with AI attribution"
echo ""
echo "Hooks are versioned in .githooks/ - changes take effect immediately."
echo ""
echo "Remember: You are responsible for code you commit."
echo "You prompted it, you reviewed it, you own it."
