#!/bin/bash
set -euo pipefail

ISO_ROOT="iso_root"
KERNEL_BIN="target/x86_64-sex/release/sex-kernel"
DISPLAY_BIN="target/x86_64-sex/release/sexdisplay"
LIMINE_DIR="limine"

echo "🧹 Step 1: Preparing ISO Root..."
rm -rf "$ISO_ROOT"
mkdir -p "$ISO_ROOT/boot/limine"
mkdir -p "$ISO_ROOT/servers"

echo "📦 Step 2: Staging Kernel and Servers..."
cp "$KERNEL_BIN" "$ISO_ROOT/boot/sex-kernel"
cp "$DISPLAY_BIN" "$ISO_ROOT/servers/sexdisplay"

echo "📀 Step 3: Injecting Boot Records (Limine 7.x)..."
cp "$LIMINE_DIR/limine-bios.sys" "$ISO_ROOT/boot/limine/"
cp "$LIMINE_DIR/limine-cd.bin" "$ISO_ROOT/boot/limine/"
cp "$LIMINE_DIR/limine-eltorito-efi.bin" "$ISO_ROOT/boot/limine/"

echo "📝 Step 4: Generating limine.conf..."
cat > "$ISO_ROOT/boot/limine/limine.conf" << 'CONF_EOF'
TIMEOUT=3
SERIAL=yes

:SexOS SASOS
    PROTOCOL=limine
    KERNEL_PATH=boot():/boot/sex-kernel
    MODULE_PATH=boot():/servers/sexdisplay
    RESOLUTION=1280x720x32
CONF_EOF

# Compatibility links
cp "$ISO_ROOT/boot/limine/limine.conf" "$ISO_ROOT/limine.conf"

echo "💿 Step 5: Executing xorriso Synthesis..."
xorriso -as mkisofs -b boot/limine/limine-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --protective-msdos-label \
        "$ISO_ROOT" -o sexos-sasos.iso

echo "🔏 Step 6: Finalizing BIOS Partition (The Magic Step)..."
"./$LIMINE_DIR/limine" bios-install sexos-sasos.iso

echo "✅ SUCCESS: sexos-sasos.iso is ready for hardware enforcement."
