#!/bin/bash
# SASOS Rust-Native Build Pipeline

# 1. Setup paths
ROOT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )/.." && pwd )"
TARGET_DIR="$ROOT_DIR/target/x86_64-unknown-none/release"
ISO_DIR="$ROOT_DIR/build/iso_root"

cd "$ROOT_DIR" || exit

# 2. Compile everything using the Nightly toolchain
echo "--- Compiling Rust Microkernel & Servers ---"
cargo build --release --target x86_64-unknown-none

# 3. Prep the ISO structure
mkdir -p "$ISO_DIR/boot"
mkdir -p "$ISO_DIR/servers"

# 4. Copy the Kernel
cp "$TARGET_DIR/sex-kernel" "$ISO_DIR/boot/kernel.bin"

# 5. Copy the Rust Servers (Mapping them to the expected locations)
# Add any missing servers to this list
for server in sexc sexnode sexnet sexfiles sexdrive sexgemini sexdisplay sextuxedo sexstore sexinput; do
    if [ -f "$TARGET_DIR/$server" ]; then
        echo "Packing $server..."
        mkdir -p "$ISO_DIR/servers/$server/bin"
        cp "$TARGET_DIR/$server" "$ISO_DIR/servers/$server/bin/$server"
    else
        echo "Warning: $server not found in target directory!"
    fi
done

# 6. Generate the bootable ISO
echo "--- Generating sexos-v1.0.0.iso ---"
xorriso -as mkisofs \
    -R -b boot/grub/stage2_eltorito \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    -o sexos-v1.0.0.iso "$ISO_DIR"
