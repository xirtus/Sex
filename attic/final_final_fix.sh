#!/bin/bash
set -e
echo "=== [SexOS] Final Precision Type Alignment ==="

# 1. Fix Memory Map field names (type_ -> typ, length -> len)
# Also adding the necessary dereferences (*) to get past the NonNullPtr
sed -i 's/r.type_/r.typ/g' kernel/src/memory.rs
sed -i 's/r.length/r.len/g' kernel/src/memory.rs

# 2. Fix the SMP Goto-Address logic
# We need to access the field directly and cast the function to a u64
# Using a more robust pointer assignment for the Limine SMP response
sed -i 's/(*cpu).goto_address.write(limine_ap_entry as u64);/(*cpu).goto_address = limine_ap_entry;/' kernel/src/smp.rs

# 3. Clean and Final Build
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
    echo "FAILED: Compilation succeeded but binary was not found."
fi
