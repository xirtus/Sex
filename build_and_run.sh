#!/bin/bash
set -e

echo "Building Sex SASOS microkernel and servers..."
export PATH="/Users/xirtus/.cargo/bin:$PATH"
cargo build --release

BIN_DIR="target/x86_64-unknown-none/release"
KERNEL="sex-kernel"
SERVERS=("sexc" "sext" "sexfiles" "sexnode" "sexgemini" "sexnet" "sexstore" "sexinput" "sexdisplay" "sexdrive")

echo "Verifying ELF binaries..."
if ! file "$BIN_DIR/$KERNEL" | grep -q "ELF 64-bit LSB"; then
    echo "Error: $KERNEL is not a valid x86_64 ELF!"
    exit 1
fi

for server in "${SERVERS[@]}"; do
    if ! file "$BIN_DIR/$server" | grep -q "ELF 64-bit LSB"; then
        echo "Error: $server is not a valid x86_64 ELF!"
        exit 1
    fi
done

echo "Bundling servers into initrd.img..."
mkdir -p initrd_root
for server in "${SERVERS[@]}"; do
    cp "$BIN_DIR/$server" initrd_root/
done

(cd initrd_root && find . | cpio -o -H newc > ../initrd.img)
rm -rf initrd_root

echo "Launching QEMU (Direct Boot)..."
qemu-system-x86_64 \
    -kernel "$BIN_DIR/$KERNEL" \
    -initrd "initrd.img" \
    -machine q35 \
    -cpu max,pku=on \
    -m 2G \
    -serial stdio \
    -display none \
    -no-reboot
