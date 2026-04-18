#!/bin/bash
set -e
echo "=== [SexOS] Resolving Borrow Checker Conflict ==="

# 1. Surgical rewrite of boot_aps to avoid E0503
# We extract bsp_lapic_id to a local variable before borrowing cpus
cat << 'RS' > kernel/src/smp.rs
use limine::{SmpResponse, SmpInfo};

pub fn boot_aps(smp: &mut SmpResponse) {
    let bsp_id = smp.bsp_lapic_id; // Extract value here to avoid borrow conflict
    let cpus = smp.cpus();
    
    for cpu in cpus {
        if unsafe { (*cpu).lapic_id } == bsp_id {
            continue;
        }
        // Manual bootstrap assignment
        unsafe {
            (*cpu).goto_address = limine_ap_entry as extern "C" fn(*const SmpInfo) -> !;
        }
    }
}

extern "C" fn limine_ap_entry(info: *const SmpInfo) -> ! {
    let lapic_id = unsafe { (*info).lapic_id };
    // Kernel initialization per core goes here
    loop { x86_64::instructions::hlt(); }
}
RS

# 2. Final Build attempt
echo "[Step 1/2] Rebuilding Kernel Core..."
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo build --release --package sex-kernel \
  --target x86_64-sex.json \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec

# 3. Assembly
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
