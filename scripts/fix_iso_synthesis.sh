#!/bin/bash
set -euo pipefail

ISO_ROOT="iso_root"
KERNEL_BIN="target/x86_64-sex/release/sex-kernel"
DISPLAY_BIN="target/x86_64-sex/release/sexdisplay"

echo "🧹 Step 1: Cleaning and Rebuilding ISO Root..."
rm -rf "$ISO_ROOT"
mkdir -p "$ISO_ROOT/boot"
mkdir -p "$ISO_ROOT/boot/limine"
mkdir -p "$ISO_ROOT/servers"

echo "📦 Step 2: Deploying Kernel and Modules..."
cp "$KERNEL_BIN" "$ISO_ROOT/boot/sex-kernel"
cp "$DISPLAY_BIN" "$ISO_ROOT/servers/sexdisplay"

# Also include the Limine binaries if they are in your root
# cp limine-bios.sys limine-cd.bin limine-eltorito-efi.bin "$ISO_ROOT/boot/limine/"

echo "📝 Step 3: Generating Limine 7.x Configuration (limine.conf)..."
# We generate both .cfg and .conf to be 100% safe
cat > "$ISO_ROOT/boot/limine/limine.conf" << 'CONF_EOF'
TIMEOUT=3

:SexOS SASOS
    PROTOCOL=limine
    KERNEL_PATH=boot():/boot/sex-kernel
    MODULE_PATH=boot():/servers/sexdisplay
    RESOLUTION=1280x720x32
CONF_EOF

# Mirror to the root and to .cfg extension for redundancy
cp "$ISO_ROOT/boot/limine/limine.conf" "$ISO_ROOT/limine.conf"
cp "$ISO_ROOT/boot/limine/limine.conf" "$ISO_ROOT/boot/limine/limine.cfg"

echo "💿 Step 4: Minting the Hardware ISO via xorriso..."
xorriso -as mkisofs -b boot/limine/limine-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --protective-msdos-label \
        "$ISO_ROOT" -o sexos-sasos.iso

# Re-run the bios-install if necessary
# ./limine bios-install sexos-sasos.iso

echo "✅ ISO SYNTHESIS COMPLETE: sexos-sasos.iso is ready."
