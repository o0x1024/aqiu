#!/bin/bash
# Set system DNS to a public DNS server (for TUN mode)
# Usage: set_dns.sh <IP address>
# NOTE: This script does NOT require sudo - networksetup can modify DNS without root

# Validate IPv4 address format
function is_valid_ipv4() {
    local ip=$1
    local IFS='.'
    local -a octets

    [[ ! $ip =~ ^([0-9]+\.){3}[0-9]+$ ]] && return 1
    read -r -a octets <<<"$ip"
    [ "${#octets[@]}" -ne 4 ] && return 1

    for octet in "${octets[@]}"; do
        if ! [[ "$octet" =~ ^[0-9]+$ ]] || ((octet < 0 || octet > 255)); then
            return 1
        fi
    done
    return 0
}

# Validate IPv6 address format
function is_valid_ipv6() {
    local ip=$1
    if [[ ! $ip =~ ^([0-9a-fA-F]{0,4}:){1,7}[0-9a-fA-F]{0,4}$ ]] &&
        [[ ! $ip =~ ^(([0-9a-fA-F]{0,4}:){0,7}:|(:[0-9a-fA-F]{0,4}:){0,6}:[0-9a-fA-F]{0,4})$ ]]; then
        return 1
    fi
    return 0
}

# Validate IP address (IPv4 or IPv6)
function is_valid_ip() {
    is_valid_ipv4 "$1" || is_valid_ipv6 "$1"
}

# Check arguments
[ $# -lt 1 ] && echo "Usage: $0 <IP address>" && exit 1
! is_valid_ip "$1" && echo "$1 is not a valid IP address." && exit 1

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

# Get current DNS settings
original_dns=$(networksetup -getdnsservers "$hardware_port")

# Check if current DNS settings are valid
is_valid_dns=false
for ip in $original_dns; do
    ip=$(echo "$ip" | tr -d '[:space:]')
    if [ -n "$ip" ] && (is_valid_ipv4 "$ip" || is_valid_ipv6 "$ip"); then
        is_valid_dns=true
        break
    fi
done

# Store original DNS settings for later restoration
# Use USER directory to avoid permission issues (no sudo needed)
config_dir="$HOME/Library/Application Support/aqiu"
mkdir -p "$config_dir"
dns_backup_file="$config_dir/.original_dns.txt"

if [ "$is_valid_dns" = false ] || [[ "$original_dns" == *"There aren't any DNS Servers"* ]]; then
    echo "empty" >"$dns_backup_file"
else
    echo "$original_dns" >"$dns_backup_file"
fi

# Set new DNS (no sudo needed for networksetup)
networksetup -setdnsservers "$hardware_port" "$1"
echo "Set DNS to $1 for $hardware_port"
