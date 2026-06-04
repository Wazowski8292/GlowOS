#!/bin/bash
set -e

KERNEL="target/x86_64-os/debug/os"
ESP="target/x86_64-os/debug/esp"
OVMF_CODE="/usr/share/edk2/x64/OVMF_CODE.4m.fd"
OVMF_VARS="/usr/share/edk2/x64/OVMF_VARS.4m.fd"

cargo build

mkdir -p "$ESP/EFI/BOOT"
cp "$KERNEL" "$ESP/EFI/BOOT/BOOTX64.EFI"

cp "$OVMF_VARS" target/x86_64-os/debug/OVMF_VARS.fd

qemu-system-x86_64 \
    -drive if=pflash,format=raw,readonly=on,file="$OVMF_CODE" \
    -drive if=pflash,format=raw,file=target/x86_64-os/debug/OVMF_VARS.fd \
    -drive format=raw,media=disk,file=fat:rw:"$ESP" \
    -device qemu-xhci \
    -device usb-storage,drive=stick \
    -drive if=none,id=stick,file=usb.img,format=raw \
    -serial stdio