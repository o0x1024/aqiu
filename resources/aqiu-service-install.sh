#!/bin/bash
# AQiu Service Installer for macOS
# This script installs the aqiu-service as a LaunchDaemon

set -e

SERVICE_NAME="com.aqiu.service"
SERVICE_BIN_NAME="aqiu-service"
INSTALL_DIR="/Library/Application Support/aqiu"
LAUNCHDAEMONS_DIR="/Library/LaunchDaemons"
PLIST_FILE="${LAUNCHDAEMONS_DIR}/${SERVICE_NAME}.plist"

# Get script directory (where the binary should be)
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== AQiu Service Installer ==="
echo "Installing ${SERVICE_NAME}..."

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "Error: This script must be run as root (sudo)"
    exit 1
fi

# Find service binary
SERVICE_BIN=""
if [ -f "${SCRIPT_DIR}/${SERVICE_BIN_NAME}" ]; then
    SERVICE_BIN="${SCRIPT_DIR}/${SERVICE_BIN_NAME}"
elif [ -f "${SCRIPT_DIR}/../MacOS/${SERVICE_BIN_NAME}" ]; then
    SERVICE_BIN="${SCRIPT_DIR}/../MacOS/${SERVICE_BIN_NAME}"
else
    echo "Error: ${SERVICE_BIN_NAME} binary not found"
    exit 1
fi

echo "Found service binary: ${SERVICE_BIN}"

# Create install directory
mkdir -p "${INSTALL_DIR}"

# Stop and unload existing service if running
if launchctl print system/${SERVICE_NAME} &>/dev/null; then
    echo "Stopping existing service..."
    launchctl bootout system/${SERVICE_NAME} 2>/dev/null || true
fi

# Copy binary to install directory
echo "Copying service binary..."
cp "${SERVICE_BIN}" "${INSTALL_DIR}/${SERVICE_BIN_NAME}"
chmod 755 "${INSTALL_DIR}/${SERVICE_BIN_NAME}"
chown root:wheel "${INSTALL_DIR}/${SERVICE_BIN_NAME}"

# Create LaunchDaemon plist
echo "Creating LaunchDaemon plist..."
cat > "${PLIST_FILE}" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>${SERVICE_NAME}</string>
    <key>ProgramArguments</key>
    <array>
        <string>${INSTALL_DIR}/${SERVICE_BIN_NAME}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/var/log/aqiu-service.log</string>
    <key>StandardErrorPath</key>
    <string>/var/log/aqiu-service.log</string>
    <key>WorkingDirectory</key>
    <string>${INSTALL_DIR}</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>
    </dict>
</dict>
</plist>
EOF

chmod 644 "${PLIST_FILE}"
chown root:wheel "${PLIST_FILE}"

# Load and start the service
echo "Loading service..."
launchctl bootstrap system "${PLIST_FILE}"

# Verify service is running
sleep 1
if launchctl print system/${SERVICE_NAME} &>/dev/null; then
    echo "✓ Service installed and running successfully!"
else
    echo "⚠ Service installed but may not be running. Check logs at /var/log/aqiu-service.log"
fi

echo "=== Installation Complete ==="

