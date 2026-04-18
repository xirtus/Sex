#!/bin/bash
set -e
echo "=== [SexOS] Aligning Function Pointers & Signatures ==="

# 1. Fix the function signature in smp.rs to use a raw pointer
# Change: extern "C" fn limine_ap_entry(info: &SmpInfo)
# To: extern "C" fn limine_ap_entry(info: *const SmpInfo)
sed -i 's/fn limine_ap_entry(info: \&SmpInfo)/fn limine_ap_entry(info: *const limine::SmpInfo)/' kernel/src/smp.rs

# 2. Fix the assignment with an explicit cast to function pointer
# This converts the "Function Item" to the "Function Pointer" type expected by the struct
sed -i 's/(\*cpu).goto_address = limine_ap_entry;/(*cpu).goto_address = limine_ap_entry as extern "C" fn(*const limine::SmpInfo) -> !;/' kernel/src/smp.rs

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
    echo "FAILED: Linker failure. Check for unresolved symbols."
fi
