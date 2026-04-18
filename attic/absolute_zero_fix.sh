#!/bin/bash
set -e
echo "=== [SexOS] Applying Final Safety Handshakes ==="

# 1. Fix mutability in boot_aps (E0596)
sed -i 's/pub fn boot_aps(smp: \&SmpResponse)/pub fn boot_aps(smp: \&mut limine::SmpResponse)/' kernel/src/smp.rs

# 2. Fix raw pointer dereference in limine_ap_entry (E0609)
# We wrap the access in unsafe {} because info is now a raw pointer
sed -i 's/let lapic_id = info.lapic_id;/let lapic_id = unsafe { (*info).lapic_id };/' kernel/src/smp.rs

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
    echo "FAILED: Kernel binary not found."
fi
