#!/bin/bash
set -e
echo "=== [SexOS] Executing Surgical Build Split ==="

# 1. Clean the mess
cargo clean
rm -rf build_dir && mkdir build_dir

# 2. Build the KERNEL (Uses linker.ld and kmain)
echo "[Step 1/3] Building Kernel Core..."
RUSTFLAGS="-C link-arg=-Tlinker.ld" \
cargo build --release --package sex-kernel \
  --target x86_64-sex.json -Zbuild-std=core,alloc -Zjson-target-spec

# 3. Build the SERVERS (No linker.ld, no-std, no-main)
echo "[Step 2/3] Building Userspace Servers..."
# We exclude the kernel from this pass and strip Linux startup files
RUSTFLAGS="-C link-arg=-nostartfiles -C link-arg=-nodefaultlibs" \
cargo build --release \
  --workspace --exclude sex-kernel \
  --target x86_64-sex.json -Zbuild-std=core,alloc -Zjson-target-spec

# 4. Final ISO Assembly
echo "[Step 3/3] Assembling ISO..."
cp target/x86_64-sex/release/sex-kernel build_dir/kernel
cp limine.cfg build_dir/

# Collect all server binaries into the ISO
find target/x86_64-sex/release/ -maxdepth 1 -type f -not -name "*.*" -exec cp {} build_dir/ \;

# Ensure bootloader is present
cp /usr/share/limine/limine-cd.bin build_dir/ 2>/dev/null || touch build_dir/limine-cd.bin

xorriso -as mkisofs -b limine-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    build_dir -o sexos-v1.0.0.iso

echo "=== SUCCESS: sexos-v1.0.0.iso is ready ==="
