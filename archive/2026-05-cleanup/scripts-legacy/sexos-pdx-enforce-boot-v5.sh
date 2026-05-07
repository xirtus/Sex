#!/bin/bash
set -euo pipefail
echo "✦ SEX MICROKERNEL SASOS — PATCHING BUILD PIPELINE"

PROJECT_ROOT="/home/xirtus_arch/Documents/microkernel"
cd "$PROJECT_ROOT"

# 1. PURGE
cargo clean --quiet
rm -rf iso_root/ *.iso 2>/dev/null || true
mkdir -p iso_root/servers/ iso_root/boot/limine

# 2. CFG
cat > limine.cfg << 'CFG_EOF'
TIMEOUT=1
:SexOS
    PROTOCOL=limine
    KERNEL_PATH=boot:///sexos-kernel
    MODULE_PATH=boot:///servers/sexdisplay
    VIDEO_MODE=1280x720,32
CFG_EOF

# 3. BUILD_PAYLOAD
cat > build_payload.sh << 'BUILD_EOF'
#!/bin/bash
mkdir -p iso_root/servers/
cp limine.cfg iso_root/limine.cfg
cp limine.cfg iso_root/
RUSTFLAGS="-C link-arg=-Tkernel/linker.ld" cargo build --package sex-kernel --target x86_64-sex.json --release
cp target/x86_64-sex/release/sex-kernel iso_root/sexos-kernel
cargo build --manifest-path servers/sexdisplay/Cargo.toml --target x86_64-sex.json --release
cp target/x86_64-sex/release/sexdisplay iso_root/servers/sexdisplay
BUILD_EOF
chmod +x build_payload.sh

# 4. MAKEFILE
cat > Makefile << 'MAKE_EOF'
iso:
	rm -f sexos-v1.0.0.iso
	xorriso -as mkisofs -R -r -J \
	  -b boot/limine/limine-bios-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table \
	  --efi-boot boot/limine/limine-uefi-cd.bin -efi-boot-part --efi-boot-image --protective-msdos-label \
	  iso_root -o sexos-v1.0.0.iso
run-sasos:
	qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku -cdrom sexos-v1.0.0.iso -serial stdio -boot d -display gtk
MAKE_EOF

# 5. STAGING
cp limine/limine-bios-cd.bin iso_root/boot/limine/ 2>/dev/null || true
cp limine/limine-uefi-cd.bin iso_root/boot/limine/ 2>/dev/null || true
cp limine/limine-bios.sys iso_root/boot/limine/ 2>/dev/null || true

# 6. RUN
bash build_payload.sh
make iso
echo "--- AUDIT ---"
7z l sexos-v1.0.0.iso | grep -E 'limine.cfg|sexos-kernel|sexdisplay|linitrd' || true
7z x -so sexos-v1.0.0.iso limine.cfg 2>/dev/null || true
make run-sasos
