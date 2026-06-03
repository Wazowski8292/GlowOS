#!/bin/bash

set -e

OS_IMAGE="target/x86_64-os/debug/bootimage-os.bin" 

# Check if script is run as root
if [ "$EUID" -ne 0 ]; then
  echo "Please run this script as root (using sudo)."
  exit 1
fi

# List available drives to help the user identify their USB
echo "=================================================="
echo "Available disk drives on your system:"
echo "=================================================="
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    lsblk -d -o NAME,SIZE,MODEL,TRAN | grep usb || lsblk
elif [[ "$OSTYPE" == "darwin"* ]]; then
    diskutil list external physical
fi
echo "=================================================="

# Prompt user for the target USB drive
read -p "Enter the target USB drive identifier (e.g., sdb, sdc, or disk2): " TARGET_INPUT

# Clean up input path
TARGET_INPUT=$(basename "$TARGET_INPUT")

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    TARGET_DEVICE="/dev/$TARGET_INPUT"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    TARGET_DEVICE="/dev/r$TARGET_INPUT" # Using raw disk 'r' on macOS for faster writing
fi

# Verify device exists
if [ ! -b "$TARGET_DEVICE" ] && [ ! -c "$TARGET_DEVICE" ]; then
    echo "Error: Device $TARGET_DEVICE not found."
    exit 1
fi

# Final warning confirmation
echo ""
echo "CRITICAL WARNING: You are about to completely format and overwrite $TARGET_DEVICE."
echo "ALL DATA ON THIS DRIVE WILL BE PERMANENTLY LOST."
read -p "Are you absolutely sure you want to proceed? (type 'YES' to confirm): " CONFIRM

if [ "$CONFIRM" != "YES" ]; then
    echo "Deployment cancelled."
    exit 1
fi

# Ensure the OS image exists
if [ ! -f "$OS_IMAGE" ]; then
    echo "Error: Compiled OS image not found at $OS_IMAGE."
    echo "Please build your project first (e.g., cargo bootimage)."
    exit 1
fi

# Unmount the drive if it is automatically mounted by the OS
echo "Unmounting target device..."
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    sudo umount "${TARGET_DEVICE}"* 2>/dev/null || true
elif [[ "$OSTYPE" == "darwin"* ]]; then
    diskutil unmountDisk "$TARGET_DEVICE"
fi

# Write the OS binary directly to the drive (RAW format for bare-metal booting)
echo "Writing GlowOS to $TARGET_DEVICE..."
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    sudo dd if="$OS_IMAGE" of="$TARGET_DEVICE" bs=4M status=progress conv=fdatasync
elif [[ "$OSTYPE" == "darwin"* ]]; then
    sudo dd if="$OS_IMAGE" of="$TARGET_DEVICE" bs=1m status=progress
fi

echo "Flushing write buffers..."
sync

echo "Deployment complete! Your USB is now ready to boot GlowOS."