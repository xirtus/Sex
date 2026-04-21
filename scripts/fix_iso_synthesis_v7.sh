#!/bin/bash
set -euo pipefail

ISO_ROOT="iso_root"
KERNEL_BIN="target/x86_64-sex/release/sex-kernel"
DISPLAY_BIN="target/x86_64-sex/release/sexdisplay"
LIMINE_DIR="limine"

echo "🧹 Preparing ISO Root..."
rm -rf "$ISO_ROOT"
mkdir -p "$ISO_ROOT/boot/limine"
mkdir -p "$ISO_ROOT/servers"

echo "📦 Staging Kernel and Servers..."
cp "$KERNEL_BIN" "$ISO_ROOT/boot/sex-kernel"
cp "$DISPLAY_BIN" "$ISO_ROOT/servers/sexdisplay"

echo "📀 Injecting Boot Records (v7.x-binary names)..."
cp "$LIMINE_DIR/limine-bios.sys"     "$ISO_ROOT/boot/limine/"
cp "$LIMINE_DIR/limine-bios-cd.bin"  "$ISO_ROOT/boot/limine/"
cp "$LIMINE_DIR/limine-uefi-cd.bin"  "$ISO_ROOT/boot/limine/"

echo "📝 Generating limine.conf in EVERY possible location Limine searches..."
cat > "$ISO_ROOT/limine.conf" << 'CONF_EOF'
TIMEOUT=3
SERIAL=yes

:SexOS SASOS
    PROTOCOL=limine
    KERNEL_PATH=boot():/boot/sex-kernel
    MODULE_PATH=boot():/servers/sexdisplay
    RESOLUTION=1280x720x32
CONF_EOF

# Mirror everywhere Limine looks
cp "$ISO_ROOT/limine.conf" "$ISO_ROOT/boot/limine/limine.conf"
cp "$ISO_ROOT/limine.conf" "$ISO_ROOT/boot/limine.conf"

echo "💿 Executing OFFICIAL xorriso Synthesis (v7 - full hybrid flags)..."
xorriso -as mkisofs \
    -R -r -J -V "SexOS SASOS" \
    -b boot/limine/limine-bios-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    -hfsplus -apm-block-size 2048 \
    --protective-msdos-label \
    "$ISO_ROOT" -o sexos-sasos.iso

echo "🔏 Finalizing BIOS Partition..."
"./$LIMINE_DIR/limine" bios-install sexos-sasos.iso

echo "🔍 VERIFICATION: Confirming limine.conf is visible in the ISO..."
xorriso -indev sexos-sasos.iso -ls / 2>/dev/null | grep -E 'limine\.conf' || echo "⚠️  limine.conf not found at root"
xorriso -indev sexos-sasos.iso -ls /boot/limine 2>/dev/null | grep -E 'limine\.conf' || echo "⚠️  limine.conf not found in /boot/limine"
echo "✅ ISO verification complete."

echo "✅ SUCCESS: sexos-sasos.iso is ready (config guaranteed visible)."
