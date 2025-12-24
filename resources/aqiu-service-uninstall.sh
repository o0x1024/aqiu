#!/bin/bash
# AQiu Service Uninstaller for macOS
# This script removes the aqiu-service LaunchDaemon

set -e

SERVICE_NAME="com.aqiu.service"
SERVICE_BIN_NAME="aqiu-service"
INSTALL_DIR="/Library/Application Support/aqiu"
LAUNCHDAEMONS_DIR="/Library/LaunchDaemons"
PLIST_FILE="${LAUNCHDAEMONS_DIR}/${SERVICE_NAME}.plist"
SOCKET_FILE="/var/run/aqiu-service.sock"

echo "=== AQiu Service Uninstaller ==="
echo "Removing ${SERVICE_NAME}..."

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "Error: This script must be run as root (sudo)"
    exit 1
fi

# Stop and unload service
if launchctl print system/${SERVICE_NAME} &>/dev/null; then
    echo "Stopping service..."
    launchctl bootout system/${SERVICE_NAME} 2>/dev/null || true
    sleep 1
fi

# Remove plist file
if [ -f "${PLIST_FILE}" ]; then
    echo "Removing plist file..."
    rm -f "${PLIST_FILE}"
fi

# Remove service binary
if [ -f "${INSTALL_DIR}/${SERVICE_BIN_NAME}" ]; then
    echo "Removing service binary..."
    rm -f "${INSTALL_DIR}/${SERVICE_BIN_NAME}"
fi

# Remove socket file
if [ -S "${SOCKET_FILE}" ]; then
    echo "Removing socket file..."
    rm -f "${SOCKET_FILE}"
fi

# Check if install directory is empty and remove if so
if [ -d "${INSTALL_DIR}" ] && [ -z "$(ls -A "${INSTALL_DIR}")" ]; then
    echo "Removing empty install directory..."
    rmdir "${INSTALL_DIR}"
fi

echo "✓ Service uninstalled successfully!"
echo "=== Uninstallation Complete ==="

