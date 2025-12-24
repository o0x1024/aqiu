#!/bin/bash
# Install Mihomo Service Mode for AQiu
# This script requires sudo and should be run with admin privileges
# Usage: sudo install-service.sh <binary_src> <plist_content_file> <user>

set -e

SYSTEM_DIR="/Library/Application Support/aqiu"
BINARY_DST="$SYSTEM_DIR/mihomo"
PLIST_PATH="/Library/LaunchDaemons/com.aqiu.service.plist"
CONFIG_PATH="$SYSTEM_DIR/config.yaml"
SERVICE_LABEL="com.aqiu.service"

BINARY_SRC="$1"
PLIST_SRC="$2"
USER="$3"

if [ -z "$BINARY_SRC" ] || [ -z "$PLIST_SRC" ] || [ -z "$USER" ]; then
    echo "Usage: $0 <binary_src> <plist_content_file> <user>"
    exit 1
fi

if [ ! -f "$BINARY_SRC" ]; then
    echo "Binary not found: $BINARY_SRC"
    exit 1
fi

if [ ! -f "$PLIST_SRC" ]; then
    echo "Plist file not found: $PLIST_SRC"
    exit 1
fi

echo "Installing AQiu Service Mode..."

# Create system directory
mkdir -p "$SYSTEM_DIR"

# Stop existing service if running
launchctl bootout "system/$SERVICE_LABEL" 2>/dev/null || true
launchctl unload -w "$PLIST_PATH" 2>/dev/null || true

# Copy binary
cp "$BINARY_SRC" "$BINARY_DST"
chown root:wheel "$BINARY_DST"
chmod 755 "$BINARY_DST"

# Copy plist
cp "$PLIST_SRC" "$PLIST_PATH"
chown root:wheel "$PLIST_PATH"
chmod 644 "$PLIST_PATH"

# Create config if not exists
if [ ! -f "$CONFIG_PATH" ]; then
    echo "mixed-port: 7890" > "$CONFIG_PATH"
fi
chown "$USER:staff" "$CONFIG_PATH"
chmod 644 "$CONFIG_PATH"

# Set directory ownership
chown "$USER:staff" "$SYSTEM_DIR"

# Load service
launchctl bootstrap system "$PLIST_PATH" || launchctl load -w "$PLIST_PATH"

echo "Service installed successfully"

