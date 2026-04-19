#!/bin/bash
set -e

echo "--- 1. Preparing ISO Root ---"
mkdir -p iso_root
cp target/x86_64-sex/release/sex-kernel iso_root/
# Ensure Limine binaries and config are present (assuming they are in your repo root)
cp limine.cfg limine/limine-bios.sys limine/limine-bios-cd.bin limine/limine-uefi-cd.bin iso_root/ 2>/dev/null || echo "Limine assets already in place or managed by Makefile."

echo "--- 2. Generating Bootable ISO (xorriso) ---"
xorriso -as mkisofs -b limine-bios-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --efi-boot limine-uefi-cd.bin \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        iso_root -o sexos-v1.0.0.iso

echo "--- 3. Launching SASOS in QEMU (M1 Hardware Bridge) ---"
# We use -cpu max to allow TCG to synthesize PKU/MPK instructions on ARM64
qemu-system-x86_64 -cdrom sexos-v1.0.0.iso \
                   -serial stdio \
                   -m 512M \
                   -vga std \
                   -cpu max,+pku \
                   -device intel-hda -device hda-duplex \
                   -net none \
                   -monitor telnet:127.0.0.1:4444,server,nowait
