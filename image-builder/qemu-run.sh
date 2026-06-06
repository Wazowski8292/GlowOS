#!/bin/bash
set -e

OVMF_CODE="/usr/share/edk2/x64/OVMF_CODE.4m.fd"
OVMF_VARS="/usr/share/edk2/x64/OVMF_VARS.4m.fd"
OS_IMAGE="target/uefi.img"
USB_IMAGE="usb.img"

mkdir -p target

cp "$OVMF_VARS" target/OVMF_VARS.fd

if [ ! -f "$USB_IMAGE" ]; then
    echo "Creating blank usb.img (64MB)..."
    dd if=/dev/zero of="$USB_IMAGE" bs=1M count=64 status=progress
fi

qemu-system-x86_64 \
  -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
  -drive if=pflash,format=raw,file=target/OVMF_VARS.fd \
  -drive format=raw,file="$OS_IMAGE" \
  -device qemu-xhci \
  -device usb-storage,drive=stick \
  -drive if=none,id=stick,format=raw,file="$USB_IMAGE" \
  -m 256M \
  -serial stdio \
  -vga std \
  -display sdl