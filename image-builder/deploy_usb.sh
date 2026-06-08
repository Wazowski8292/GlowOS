#!/bin/bash
set -e

# ═══════════════════════════════════════════════════════════════
#  GlowOS — Build, Test & Deploy Script
# ═══════════════════════════════════════════════════════════════

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

OS_IMAGE="$PROJECT_ROOT/target/uefi.img"
OVMF_CODE="/usr/share/edk2/x64/OVMF_CODE.4m.fd"
OVMF_VARS="/usr/share/edk2/x64/OVMF_VARS.4m.fd"
OVMF_VARS_COPY="$SCRIPT_DIR/target/OVMF_VARS.fd"

# ── Build & Test (must NOT be root) ─────────────────────────────
if [ "$EUID" -ne 0 ]; then

    # ── 1. Build ────────────────────────────────────────────────
    echo "════════════════════════════════════════════════════════════"
    echo "  GlowOS — Building kernel + packaging UEFI image..."
    echo "════════════════════════════════════════════════════════════"

    cd "$PROJECT_ROOT"
    make build

    if [ ! -f "$OS_IMAGE" ]; then
        echo ""
        echo "Error: Build succeeded but image not found at $OS_IMAGE"
        echo "Check the output path inside build/src/main.rs"
        exit 1
    fi

    echo ""
    echo "✓ Kernel packaged successfully → $OS_IMAGE"

    # ── 2. Validate UEFI image structure ────────────────────────
    echo ""
    echo "════════════════════════════════════════════════════════════"
    echo "  Validating UEFI image structure..."
    echo "════════════════════════════════════════════════════════════"

    GPT_SIG=$(dd if="$OS_IMAGE" bs=1 skip=512 count=8 2>/dev/null | xxd -p 2>/dev/null || true)
    if [ "$GPT_SIG" != "4546492050415254" ]; then
        echo "⚠  Warning: Image does not appear to have a valid GPT header."
        echo "   Real UEFI firmware is stricter than QEMU — this may not boot on hardware."
        echo "   Expected GPT signature at offset 512; got: $GPT_SIG"
        echo ""
        read -p "Continue anyway? (y/n): " IGNORE_GPT
        if [ "$IGNORE_GPT" != "y" ] && [ "$IGNORE_GPT" != "Y" ]; then
            echo "Aborted."
            exit 1
        fi
    else
        echo "✓ GPT partition table detected."
    fi

    # ── 3. Test in QEMU (optional) ──────────────────────────────
    echo ""
    echo "════════════════════════════════════════════════════════════"
    echo "  Test in QEMU"
    echo "════════════════════════════════════════════════════════════"
    read -p "Test in QEMU before deploying to USB? (y/n): " TEST

    if [ "$TEST" = "y" ] || [ "$TEST" = "Y" ]; then
        if [ ! -f "$OVMF_CODE" ]; then
            echo "Error: OVMF firmware not found at $OVMF_CODE"
            echo "Install it with: sudo pacman -S edk2-ovmf  (or your distro equivalent)"
            exit 1
        fi

        mkdir -p "$SCRIPT_DIR/target"
        cp "$OVMF_VARS" "$OVMF_VARS_COPY"

        echo ""
        echo "Launching QEMU... (close the window or press Ctrl+C to exit)"
        echo ""
        "$SCRIPT_DIR/qemu-run.sh"

        echo ""
        read -p "Continue to USB deploy? (y/n): " CONTINUE
        if [ "$CONTINUE" != "y" ] && [ "$CONTINUE" != "Y" ]; then
            echo "Deploy cancelled."
            exit 0
        fi
    fi

    # ── 4. Hand off to root for USB deploy ──────────────────────
    echo ""
    echo "════════════════════════════════════════════════════════════"
    echo "  Deploy to USB — sudo required for raw disk access"
    echo "════════════════════════════════════════════════════════════"
    echo "Re-launching with sudo for the deploy step..."
    exec sudo bash "$SCRIPT_DIR/$(basename "$0")" --root-deploy "$OS_IMAGE"
fi

# ════════════════════════════════════════════════════════════════
#  Running as root — deploy only
#  Invoked as: sudo bash deploy.sh --root-deploy <image-path>
# ════════════════════════════════════════════════════════════════
if [ "$1" != "--root-deploy" ] || [ -z "$2" ]; then
    echo "Error: This script must not be run directly as root."
    echo "Run it as a normal user; it will sudo itself for the deploy step."
    exit 1
fi

OS_IMAGE="$2"

if [ ! -f "$OS_IMAGE" ]; then
    echo "Error: Image not found at $OS_IMAGE"
    exit 1
fi

echo ""
echo "════════════════════════════════════════════════════════════"
echo "  Deploy to USB"
echo "════════════════════════════════════════════════════════════"

echo "Available disk drives on your system:"
echo "════════════════════════════════════════════════════════════"
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    lsblk -d -o NAME,SIZE,MODEL,TRAN | grep -i usb || lsblk -d -o NAME,SIZE,MODEL
elif [[ "$OSTYPE" == "darwin"* ]]; then
    diskutil list external physical
else
    echo "Unsupported OS: $OSTYPE"; exit 1
fi
echo "════════════════════════════════════════════════════════════"

echo ""
read -p "Enter the target USB drive identifier (e.g., sdb, sdc, or disk2): " TARGET_INPUT
TARGET_INPUT=$(basename "$TARGET_INPUT")

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    TARGET_DEVICE="/dev/$TARGET_INPUT"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    TARGET_DEVICE="/dev/r$TARGET_INPUT"
fi

if [ ! -b "$TARGET_DEVICE" ] && [ ! -c "$TARGET_DEVICE" ]; then
    echo "Error: Device $TARGET_DEVICE not found or is not a block device."
    exit 1
fi

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    ROOT_DISK=$(lsblk -no PKNAME "$(findmnt -n -o SOURCE /)" 2>/dev/null | head -1)
    if [ "/dev/$ROOT_DISK" = "$TARGET_DEVICE" ]; then
        echo "Error: $TARGET_DEVICE appears to be your system disk. Aborting."
        exit 1
    fi
fi

echo ""
echo "Target device: $TARGET_DEVICE"
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    lsblk "$TARGET_DEVICE" 2>/dev/null || true
fi

IMAGE_SIZE=$(stat -c%s "$OS_IMAGE" 2>/dev/null || stat -f%z "$OS_IMAGE")
echo "Image size   : $(( IMAGE_SIZE / 1024 / 1024 )) MiB"

echo ""
echo "⚠  CRITICAL WARNING ⚠"
echo "You are about to completely overwrite $TARGET_DEVICE."
echo "ALL DATA ON THIS DRIVE WILL BE PERMANENTLY LOST."
echo ""
read -p "Type 'YES' to confirm: " CONFIRM

if [ "$CONFIRM" != "YES" ]; then
    echo "Deployment cancelled."
    exit 1
fi

# ── Unmount ──────────────────────────────────────────────────────
echo ""
echo "Unmounting $TARGET_DEVICE..."
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    umount "${TARGET_DEVICE}"?* 2>/dev/null || true
    umount "${TARGET_DEVICE}" 2>/dev/null || true
elif [[ "$OSTYPE" == "darwin"* ]]; then
    diskutil unmountDisk "$TARGET_DEVICE"
fi

# ── Flash ────────────────────────────────────────────────────────
echo "Writing GlowOS image to $TARGET_DEVICE..."
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    dd if="$OS_IMAGE" \
       of="$TARGET_DEVICE" \
       bs=512 \
       conv=fdatasync \
       oflag=direct \
       status=progress
elif [[ "$OSTYPE" == "darwin"* ]]; then
    dd if="$OS_IMAGE" \
       of="$TARGET_DEVICE" \
       bs=512
fi

echo "Flushing write buffers..."
sync

# ── Repair GPT to fill the device ───────────────────────────────
# The bootloader crate produces a minimal fixed-size image (~7 MiB).
# After flashing, the GPT backup header sits near sector 14463 instead
# of at the true last sector of the USB, causing the PMBR mismatch warning.
# sgdisk --move-second-header relocates it to the correct position.
echo ""
echo "════════════════════════════════════════════════════════════"
echo "  Repairing GPT to fill $TARGET_DEVICE..."
echo "════════════════════════════════════════════════════════════"
if command -v sgdisk &>/dev/null; then
    sgdisk --move-second-header "$TARGET_DEVICE"
    partprobe "$TARGET_DEVICE" 2>/dev/null || true
    echo "✓ GPT repaired."
else
    echo "⚠  sgdisk not found — GPT not repaired."
    echo "   Install gdisk and run manually:"
    echo "   sudo sgdisk --move-second-header $TARGET_DEVICE"
fi

# ── Verify GPT signature on device ──────────────────────────────
echo ""
echo "════════════════════════════════════════════════════════════"
echo "  Verifying GPT signature on device..."
echo "════════════════════════════════════════════════════════════"
GPT_ON_DEV=$(dd if="$TARGET_DEVICE" bs=1 skip=512 count=8 2>/dev/null | xxd -p 2>/dev/null || true)
if [ "$GPT_ON_DEV" = "4546492050415254" ]; then
    echo "✓ GPT signature confirmed on $TARGET_DEVICE"
else
    echo "⚠  GPT signature not found after write (got: $GPT_ON_DEV)"
    echo "   The image may not be a valid UEFI disk image."
fi

# ── Verify ESP contents ──────────────────────────────────────────
echo ""
echo "════════════════════════════════════════════════════════════"
echo "  Verifying EFI System Partition contents..."
echo "════════════════════════════════════════════════════════════"
mkdir -p /mnt/tmp_esp
PART=$(fdisk -l "$TARGET_DEVICE" | awk '/EFI System/{print $1; exit}')
if [ -n "$PART" ]; then
    if mount "$PART" /mnt/tmp_esp 2>/dev/null; then
        echo "✓ ESP mounted from $PART"
        if ls /mnt/tmp_esp/EFI/BOOT/BOOTX64.EFI &>/dev/null; then
            echo "✓ Bootloader found at EFI/BOOT/BOOTX64.EFI"
        else
            echo "⚠  BOOTX64.EFI not found — USB may not boot on real hardware."
            echo "   ESP contents:"
            find /mnt/tmp_esp -type f 2>/dev/null | sed 's/^/   /'
        fi
        umount /mnt/tmp_esp
    else
        echo "⚠  Could not mount $PART — check dmesg for details."
    fi
else
    echo "⚠  No EFI System Partition found on $TARGET_DEVICE"
fi
rmdir /mnt/tmp_esp 2>/dev/null || true

echo ""
echo "✓ Deployment complete! Your USB is ready to boot GlowOS."