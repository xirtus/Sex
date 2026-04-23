#!/bin/bash
set -euo pipefail
echo "✦ SEX MICROKERNEL SASOS — ASSEMBLING FINAL BOOT IMAGE"

PROJECT_ROOT="/home/xirtus_arch/Documents/microkernel"
cd "$PROJECT_ROOT"

# Ensure binary stage
mkdir -p iso_root
cp limine/limine-bios-cd.bin iso_root/ 2>/dev/null || true
cp limine/limine-uefi-cd.bin iso_root/ 2>/dev/null || true
cp limine/limine-bios.sys iso_root/ 2>/dev/null || true

# Assemble
rm -f sexos-v1.0.0.iso
xorriso -as mkisofs -R -r -J \
  -b limine-bios-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table \
  --efi-boot limine-uefi-cd.bin -efi-boot-part --efi-boot-image --protective-msdos-label \
  iso_root -o sexos-v1.0.0.iso

# Install Limine
chmod +x ./limine-install 2>/dev/null || true
./limine-install bios-install sexos-v1.0.0.iso

# Boot
qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku -cdrom sexos-v1.0.0.iso -serial stdio -boot d -display gtk
