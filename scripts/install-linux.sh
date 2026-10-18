#!/bin/bash
# Install script for somafm on Linux
# Copies the built binary, .desktop file, and icon to user-local directories
# Usage: ./scripts/install-linux.sh

set -e

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Paths
BUILD_DIR="$PROJECT_ROOT/src-tauri/target/release"
BINARY_NAME="somafm"
BINARY_PATH="$BUILD_DIR/$BINARY_NAME"
DESKTOP_TEMPLATE="$PROJECT_ROOT/somafm.desktop"
ICON_SOURCE="$PROJECT_ROOT/src-tauri/icons/icon.png"

# Installation paths
LOCAL_BIN_DIR="$HOME/.local/bin"
LOCAL_SHARE_DIR="$HOME/.local/share"
DESKTOP_DIR="$LOCAL_SHARE_DIR/applications"
ICON_DIR="$LOCAL_SHARE_DIR/icons/hicolor/128x128/apps"

# Ensure directories exist
mkdir -p "$LOCAL_BIN_DIR"
mkdir -p "$DESKTOP_DIR"
mkdir -p "$ICON_DIR"

# Check if binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    echo "Please run: pnpm tauri build --no-bundle"
    exit 1
fi

# Copy binary
echo "Installing binary to $LOCAL_BIN_DIR..."
cp "$BINARY_PATH" "$LOCAL_BIN_DIR/$BINARY_NAME"
chmod +x "$LOCAL_BIN_DIR/$BINARY_NAME"

# Copy icon
echo "Installing icon to $ICON_DIR..."
cp "$ICON_SOURCE" "$ICON_DIR/$BINARY_NAME.png"

# Install .desktop file
echo "Installing .desktop file to $DESKTOP_DIR..."
sed "s#@BIN@#$LOCAL_BIN_DIR/$BINARY_NAME#" "$DESKTOP_TEMPLATE" > "$DESKTOP_DIR/$BINARY_NAME.desktop"
chmod 644 "$DESKTOP_DIR/$BINARY_NAME.desktop"

echo "Installation complete!"
echo "The app is now available as 'somafm' in your application menu and path."
echo ""
echo "Verify installation with:"
echo "  which somafm"
echo "  somafm"
