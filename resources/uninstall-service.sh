#!/bin/bash
# Uninstall Mihomo Service Mode for AQiu
# This script requires sudo and should be run with admin privileges

set -e

SYSTEM_DIR="/Library/Application Support/aqiu"
BINARY_PATH="$SYSTEM_DIR/mihomo"
PLIST_PATH="/Library/LaunchDaemons/com.aqiu.service.plist"
SERVICE_LABEL="com.aqiu.service"

echo "Uninstalling AQiu Service Mode..."

# Stop and unload service
launchctl bootout "system/$SERVICE_LABEL" 2>/dev/null || true
launchctl unload -w "$PLIST_PATH" 2>/dev/null || true

# Remove plist
rm -f "$PLIST_PATH"

# Remove binary
rm -f "$BINARY_PATH"

echo "Service uninstalled successfully"
# Note: Config files in $SYSTEM_DIR are preserved

