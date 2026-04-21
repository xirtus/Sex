#!/bin/bash
# =================================================================
# SexOS Phase 19: Substrate Hardening (Binary Trampoline Fix)
# =================================================================
set -euo pipefail

echo "─── Step 1: Updating main.rs Trampoline ───"
cat > kernel/src/main.rs << 'MAIN_EOF'
#![no_std]
#![no_main]

use sex_kernel::kernel_init;

#[no_mangle]
extern "C" fn _start() -> ! {
    // Jump to the hardened library initialization
    kernel_init();

    // God-mode loop
    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
MAIN_EOF

echo "─── Step 2: Suppressing Compatibility Warnings in lib.rs ───"
# We add #[allow(static_mut_refs)] to keep the logs clean during development
sed -i '' 's/pub fn kernel_init()/#[allow(static_mut_refs)]\npub fn kernel_init()/' kernel/src/lib.rs || \
sed -i 's/pub fn kernel_init()/#[allow(static_mut_refs)]\npub fn kernel_init()/' kernel/src/lib.rs

echo "─── Step 3: Atomic Synthesis (The Finish Line) ───"
RUSTC_BOOTSTRAP=1 cargo build \
    --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -p sex-kernel \
    --release

echo "✅ SYSTEM SYNTHESIS SUCCESSFUL."
echo "1. Run ./scripts/final_payload.sh to mint the ISO."
echo "2. Launch QEMU: -cpu max,+pku -serial stdio"
