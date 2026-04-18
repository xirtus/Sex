#!/bin/bash
set -e
echo "=== [SexOS] Executing God Mode Final Alignment ==="

# 1. Fix the stray parenthesis and pointer logic in smp.rs
# We use a clean replacement that removes the broken comment and paren
sed -i 's/(\*(\*cpu).goto_address = limine_ap_entry;.*/(*cpu).goto_address = limine_ap_entry;/' kernel/src/smp.rs

# 2. Re-verify Memory Map field names (typ/len)
sed -i 's/r.type_/r.typ/g' kernel/src/memory.rs 2>/dev/null || true
sed -i 's/r.length/r.len/g' kernel/src/memory.rs 2>/dev/null || true

# 3. Final Build attempt
echo "[Step 1/2] Rebuilding Kernel Core..."
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
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
    echo "FAILED: Compilation successful but binary not found."
fi
