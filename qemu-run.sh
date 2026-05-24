#!/bin/bash

set -e

if [[ "$1" == *"/deps/"* ]]; then 
    bootimage runner "$1" -device qemu-xhci -device usb-storage,drive=stick -drive if=none,id=stick,file=usb.img,format=raw
else
    cargo bootimage
    qemu-system-x86_64 \
      -drive format=raw,file=target/x86_64-os/debug/bootimage-os.bin \
      -device qemu-xhci \
      -device usb-storage,drive=stick \
      -drive if=none,id=stick,file=usb.img,format=raw \
      -serial stdio
fi