#!/bin/bash
set -e

# ─────────────────────────────────────────────
#  GlowOS — Build & Deploy Script
# ─────────────────────────────────────────────

OS_IMAGE="target/x86_64-os/debug/glowos_uefi.img"
CARGO_MANIFEST="xtask/Cargo.toml"   # adjust if your xtask lives elsewhere

# ── 1. Build ──────────────────────────────────
echo "=================================================="
echo " Building kernel + packaging UEFI image…"
echo "=================================================="

# Build the kernel itself
cargo build --target x86_64-os.json -Z build-std=core,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem 2>/dev/null \
  || cargo build --target x86_64-unknown-none 2>/dev/null \
  || cargo build   # fallback – adjust target to match your project

# Run the xtask / build script that wraps the Rust packaging code
#   (the main() you pasted lives here and produces the .img)
cargo run --manifest-path "$CARGO_MANIFEST" 2>/dev/null \
  || cargo run   # fallback if no separate xtask crate

# Verify the image was produced
if [ ! -f "$OS_IMAGE" ]; then
  echo "Error: Build succeeded but image not found at $OS_IMAGE."
  echo "Check the paths inside your Rust build script."
  exit 1
fi

echo ""
echo "Kernel packaged successfully → $OS_IMAGE"

# ── 2. Deploy ─────────────────────────────────
echo ""
echo "=================================================="
echo " Deploy to USB"
echo "=================================================="

# Must be root for raw-device writes
if [ "$EUID" -ne 0 ]; then
  echo "Re-launching with sudo for the deploy step…"
  exec sudo "$0" "$@"
fi

# List drives to help the user pick the right one
echo "Available disk drives on your system:"
echo "=================================================="
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
  lsblk -d -o NAME,SIZE,MODEL,TRAN | grep usb || lsblk
elif [[ "$OSTYPE" == "darwin"* ]]; then
  diskutil list external physical
fi
echo "=================================================="

# Prompt for target device
read -p "Enter the target USB drive identifier (e.g., sdb, sdc, or disk2): " TARGET_INPUT
TARGET_INPUT=$(basename "$TARGET_INPUT")

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
  TARGET_DEVICE="/dev/$TARGET_INPUT"
elif [[ "$OSTYPE" == "darwin"* ]]; then
  TARGET_DEVICE="/dev/r$TARGET_INPUT"   # raw disk = faster on macOS
else
  echo "Unsupported OS: $OSTYPE"; exit 1
fi

# Verify the device exists
if [ ! -b "$TARGET_DEVICE" ] && [ ! -c "$TARGET_DEVICE" ]; then
  echo "Error: Device $TARGET_DEVICE not found."; exit 1
fi

# Final confirmation
echo ""
echo "⚠  CRITICAL WARNING ⚠"
echo "You are about to completely overwrite $TARGET_DEVICE."
echo "ALL DATA ON THIS DRIVE WILL BE PERMANENTLY LOST."
read -p "Type 'YES' to confirm: " CONFIRM
if [ "$CONFIRM" != "YES" ]; then
  echo "Deployment cancelled."; exit 1
fi

# Unmount before writing
echo "Unmounting $TARGET_DEVICE…"
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
  umount "${TARGET_DEVICE}"* 2>/dev/null || true
elif [[ "$OSTYPE" == "darwin"* ]]; then
  diskutil unmountDisk "$TARGET_DEVICE"
fi

# Write the image
echo "Writing GlowOS to $TARGET_DEVICE…"
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
  dd if="$OS_IMAGE" of="$TARGET_DEVICE" bs=4M status=progress conv=fdatasync
elif [[ "$OSTYPE" == "darwin"* ]]; then
  dd if="$OS_IMAGE" of="$TARGET_DEVICE" bs=1m
fi

echo "Flushing write buffers…"
sync

echo ""
echo "✓ Deployment complete! Your USB is ready to boot GlowOS."