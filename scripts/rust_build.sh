#!/bin/bash
# SASOS Rust-Native Build Pipeline (Root ISO Layout Fix)

# 1. Setup paths
ROOT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )/.." && pwd )"
TARGET_DIR="$ROOT_DIR/target/x86_64-sex/release"
ISO_DIR="$ROOT_DIR/build/iso_root"

# Inject rust-lld for Darwin
RUST_LLD_DIR="/Users/xirtus/.rustup/toolchains/nightly-aarch64-apple-darwin/lib/rustlib/aarch64-apple-darwin/bin"
export PATH="$PATH:$RUST_LLD_DIR"
export RUSTC_BOOTSTRAP=1

cd "$ROOT_DIR" || exit

# 2. Compile everything
echo "--- Compiling Rust Microkernel & Servers ---"
cargo build -Zbuild-std=core,alloc --release --target x86_64-sex.json --workspace --exclude sexbuild

# 3. Prep the ISO structure
rm -rf "$ISO_DIR"
mkdir -p "$ISO_DIR/servers"

# 4. Copy the Kernel to ROOT
if [ -f "$TARGET_DIR/sex-kernel" ]; then
    cp "$TARGET_DIR/sex-kernel" "$ISO_DIR/sexos-kernel"
else
    echo "ERROR: Kernel build failed!"
    exit 1
fi

# 4.1 Copy initrd.sex to ROOT
if [ -f "initrd.sex" ]; then
    cp "initrd.sex" "$ISO_DIR/initrd.sex"
else
    echo "Warning: initrd.sex not found!"
fi

# 5. Copy Limine binaries and config to ROOT
cp limine_bin/limine-bios.sys "$ISO_DIR/"
cp limine_bin/limine-bios-cd.bin "$ISO_DIR/"
cp limine_bin/limine-uefi-cd.bin "$ISO_DIR/"
cp limine.cfg "$ISO_DIR/"

# 6. Copy the Rust Servers
for server in sexc sexnode sexnet sexfiles sexdrive sexgemini sexdisplay tuxedo sexstore sexinput egui-hello; do
    if [ -f "$TARGET_DIR/$server" ]; then
        echo "Packing $server..."
        mkdir -p "$ISO_DIR/servers/$server/bin"
        cp "$TARGET_DIR/$server" "$ISO_DIR/servers/$server/bin/$server"
    else
        echo "Warning: $server not found in target directory!"
    fi
done

# 7. Generate the bootable ISO
echo "--- Generating sexos-v1.0.0.iso ---"
xorriso -as mkisofs \
    -b limine-bios-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    --efi-boot limine-uefi-cd.bin \
    -efi-boot-part --efi-boot-image --protective-msdos-label \
    -o sexos-v1.0.0.iso "$ISO_DIR"

# 8. Patch for BIOS boot
if [ -f "./limine/limine" ]; then
    echo "--- Patching ISO for BIOS boot ---"
    ./limine/limine bios-install sexos-v1.0.0.iso
else
    echo "Warning: limine tool not found, BIOS boot might fail!"
fi

echo "Build complete: sexos-v1.0.0.iso"
