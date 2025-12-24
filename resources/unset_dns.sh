#!/bin/bash
# Restore original system DNS (when TUN mode is disabled)
# NOTE: This script does NOT require sudo - networksetup can modify DNS without root

# Get network interface and hardware port
nic=$(route -n get default 2>/dev/null | grep "interface" | awk '{print $2}')
if [ -z "$nic" ]; then
    echo "Could not determine default network interface"
    exit 1
fi

hardware_port=$(networksetup -listnetworkserviceorder | awk -v dev="$nic" '
    /^\([0-9]+\) /{port=$0; sub(/^\([0-9]+\) /, "", port)} 
    /\(Hardware Port:/{interface=$NF;sub(/\)/, "", interface); if (interface == dev) {print port; exit}}
')

if [ -z "$hardware_port" ]; then
    echo "Could not determine hardware port for interface: $nic"
    exit 1
fi

# Restore original DNS from backup file
# Use USER directory (same as set_dns.sh)
config_dir="$HOME/Library/Application Support/aqiu"
dns_backup_file="$config_dir/.original_dns.txt"

if [ -f "$dns_backup_file" ]; then
    original_dns=$(cat "$dns_backup_file")
    if [ "$original_dns" = "empty" ]; then
        # Restore to DHCP-assigned DNS
        networksetup -setdnsservers "$hardware_port" "Empty"
        echo "Restored DNS to DHCP for $hardware_port"
    else
        # shellcheck disable=SC2086
        networksetup -setdnsservers "$hardware_port" $original_dns
        echo "Restored DNS to $original_dns for $hardware_port"
    fi
    rm -f "$dns_backup_file"
else
    # No backup file, just clear to DHCP
    networksetup -setdnsservers "$hardware_port" "Empty"
    echo "Cleared DNS to DHCP for $hardware_port (no backup found)"
fi
