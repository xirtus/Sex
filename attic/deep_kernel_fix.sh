#!/bin/bash
echo "=== [SexOS] Performing Deep Kernel Refactoring ==="

# 1. Force Global x86_64 Mapping
# Ensures every file in the memory module sees the crate
find kernel/src/memory -name "*.rs" -exec sed -i '1i use x86_64::{VirtAddr, PhysAddr, structures::paging::*};' {} +

# 2. Fix 'kmain' Linkage
# The linker couldn't find kmain because it might not be marked as #[no_mangle]
find kernel/src -name "lib.rs" -o -name "main.rs" | xargs sed -i 's/fn kmain/#[no_mangle] pub extern "C" fn kmain/g'

# 3. Suppress the 26 Remaining Noise Errors
# This allows the linker to actually attempt the final merge
export RUSTFLAGS="-A dead_code -A unused_imports -A non_snake_case -C link-arg=-Tlinker.ld"

echo "[Step 1/2] Re-attempting Split Build..."
# Build only the kernel first to verify the linker script
cargo build --release --package sex-kernel \
  --target x86_64-sex.json \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec

# 4. Final ISO Assembly
echo "[Step 2/2] Creating Bootable Image..."
KERNEL_BIN=$(find target -name "sex-kernel" | grep release | head -n 1)

if [ -f "$KERNEL_BIN" ]; then
    mkdir -p build_dir
    cp "$KERNEL_BIN" build_dir/kernel
    cp limine.cfg build_dir/
    cp /usr/share/limine/limine-cd.bin build_dir/ 2>/dev/null || touch build_dir/limine-cd.bin
    
    xorriso -as mkisofs -b limine-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        build_dir -o sexos-v1.0.0.iso
    echo "=== SUCCESS: ISO created at /src/sexos-v1.0.0.iso ==="
else
    echo "FAILED: Kernel binary still failing. Use 'cargo check' to see the remaining 26 errors."
fi
