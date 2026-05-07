#!/bin/bash
set -euo pipefail
echo "✦ SEX MICROKERNEL SASOS — MANUAL HYBRID ISO BUILD"

PROJECT_ROOT="/home/xirtus_arch/Documents/microkernel"
cd "$PROJECT_ROOT"

# 1. PURGE
cargo clean --quiet
rm -rf iso_root/ *.iso *.iso.tmp 2>/dev/null || true
mkdir -p iso_root/servers/ iso_root/boot/limine iso_root/limine

# 2. CFG
cat > limine.cfg << 'CFG_EOF'
TIMEOUT=1

:SexOS
    PROTOCOL=limine
    KERNEL_PATH=boot:///sexos-kernel
    MODULE_PATH=boot:///servers/sexdisplay
    VIDEO_MODE=1280x720,32
CFG_EOF

# 3. PLACE
cp limine.cfg iso_root/limine.cfg
cp limine.cfg iso_root/boot/limine.cfg
cp limine.cfg iso_root/boot/limine/limine.cfg
cp limine.cfg iso_root/limine/limine.cfg

# 4. BUILD
bash build_payload.sh
cp target/x86_64-sex/release/sex-kernel iso_root/sexos-kernel
cp target/x86_64-sex/release/sexdisplay iso_root/servers/sexdisplay

# 5. LIMINE BINARIES
[ -d limine ] && cp limine/limine-bios-cd.bin iso_root/boot/limine/ || true
[ -d limine ] && cp limine/limine-uefi-cd.bin iso_root/boot/limine/ || true
[ -d limine ] && cp limine/limine-bios.sys iso_root/boot/limine/ || true

# 6. HYBRID ISO
rm -f sexos-v1.0.0.iso
xorriso -as mkisofs \
  -R -r -J \
  -b boot/limine/limine-bios-cd.bin \
  -no-emul-boot -boot-load-size 4 -boot-info-table \
  --efi-boot boot/limine/limine-uefi-cd.bin \
  -efi-boot-part --efi-boot-image --protective-msdos-label \
  iso_root -o sexos-v1.0.0.iso

# 7. AUDIT
echo "--- AUDIT ---"
if command -v 7z >/dev/null 2>&1; then
    7z l sexos-v1.0.0.iso | grep -E 'limine.cfg|sexos-kernel|sexdisplay|linitrd'
    7z x -so sexos-v1.0.0.iso limine.cfg 2>/dev/null || true
fi

# 8. RUN
qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku -cdrom sexos-v1.0.0.iso -serial stdio -boot d -display gtk
