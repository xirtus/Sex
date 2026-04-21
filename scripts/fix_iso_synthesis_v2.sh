#!/bin/bash
set -euo pipefail

ISO_ROOT="iso_root"
KERNEL_BIN="target/x86_64-sex/release/sex-kernel"
DISPLAY_BIN="target/x86_64-sex/release/sexdisplay"

echo "🔍 Step 1: Locating Limine Binary Assets..."
# We search for the limine directory. Assuming it's in the root.
LIMINE_DIR=$(find . -maxdepth 2 -type d -name "limine" | head -n 1)

if [ -z "$LIMINE_DIR" ]; then
    echo "❌ ERROR: Limine directory not found. Please ensure you have the limine binaries in your project root."
    exit 1
fi

echo "🧹 Step 2: Cleaning and Rebuilding ISO Root..."
rm -rf "$ISO_ROOT"
mkdir -p "$ISO_ROOT/boot/limine"
mkdir -p "$ISO_ROOT/servers"

echo "📦 Step 3: Deploying Kernel and Modules..."
cp "$KERNEL_BIN" "$ISO_ROOT/boot/sex-kernel"
cp "$DISPLAY_BIN" "$ISO_ROOT/servers/sexdisplay"

echo "📀 Step 4: Injecting Limine Boot Records..."
# These are the files xorriso needs to find INSIDE the ISO image
cp "$LIMINE_DIR/limine-bios.sys" "$ISO_ROOT/boot/limine/"
cp "$LIMINE_DIR/limine-cd.bin" "$ISO_ROOT/boot/limine/"
cp "$LIMINE_DIR/limine-eltorito-efi.bin" "$ISO_ROOT/boot/limine/"

echo "📝 Step 5: Generating Limine Configuration..."
cat > "$ISO_ROOT/boot/limine/limine.conf" << 'CONF_EOF'
TIMEOUT=3
SERIAL=yes

:SexOS SASOS
    PROTOCOL=limine
    KERNEL_PATH=boot():/boot/sex-kernel
    MODULE_PATH=boot():/servers/sexdisplay
    RESOLUTION=1280x720x32
CONF_EOF

# Compatibility mirrors
cp "$ISO_ROOT/boot/limine/limine.conf" "$ISO_ROOT/limine.conf"
cp "$ISO_ROOT/boot/limine/limine.conf" "$ISO_ROOT/boot/limine/limine.cfg"

echo "💿 Step 6: Minting the Hardware ISO via xorriso..."
xorriso -as mkisofs -b boot/limine/limine-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --protective-msdos-label \
        "$ISO_ROOT" -o sexos-sasos.iso

echo "🔏 Step 7: Finalizing BIOS Partition Table..."
# This makes the ISO bootable on legacy BIOS systems
"$LIMINE_DIR/limine" bios-install sexos-sasos.iso

echo "✅ SUCCESS: sexos-sasos.iso is synthesized and bootable."
