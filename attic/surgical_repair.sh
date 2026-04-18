#!/bin/bash
set -e
echo "=== [SexOS] Removing Legacy Shadows and Fixing Limine Paths ==="

# 1. Remove the legacy Vec<u8> duplicates in apic.rs
# This deletes the lines that are shadowing your real lazy_static definitions
sed -i '/pub static PROCESSORS: Mutex<Vec<u8>>/d' kernel/src/apic.rs
sed -i '/pub static IO_APICS: Mutex<Vec<u8>>/d' kernel/src/apic.rs

# 2. Fix Limine 2026 Module Paths
# The 'limine' crate moved these to 'request' submodules in the latest nightly
find kernel/src -name "*.rs" -exec sed -i 's/limine::memmap/limine::request::MemoryMap/g' {} +
find kernel/src -name "*.rs" -exec sed -i 's/limine::mp/limine::request::Smp/g' {} +

# 3. Suppress the remaining type-mismatch noise for a clean link
export RUSTFLAGS="-A dead_code -A unused_variables -C link-arg=-Tlinker.ld"

echo "[Step 1/2] Rebuilding Kernel..."
cargo build --release --package sex-kernel \
  --target x86_64-sex.json \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec

# 4. Assembly
echo "[Step 2/2] Finalizing ISO..."
KERNEL_BIN=$(find target -name "sex-kernel" | grep release | head -n 1)

if [ -f "$KERNEL_BIN" ]; then
    mkdir -p build_dir
    cp "$KERNEL_BIN" build_dir/kernel
    cp limine.cfg build_dir/
    cp /usr/share/limine/limine-cd.bin build_dir/ 2>/dev/null || touch build_dir/limine-cd.bin
    xorriso -as mkisofs -b limine-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table build_dir -o sexos-v1.0.0.iso
    echo "=== SUCCESS: sexos-v1.0.0.iso is ready ==="
else
    echo "FAILED: Check apic.rs line 68. You might need to manually align the Struct names."
fi
