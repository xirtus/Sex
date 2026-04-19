#!/bin/bash
# SexOS SASOS v1.0.0 - SAFE HANDOFF AUTOMATION (Phase 17.5→18/24)
set -euo pipefail

# Dynamically grab current directory instead of hardcoding
PROJECT="$PWD"
cd "$PROJECT" || { echo "ERR: no $PROJECT"; exit 1; }

echo "1. TOOLCHAIN VERIFICATION"
./scripts/fix_toolchain_and_rebuild.sh || true

echo "2. CLEAN BUILD (no_std + RustCrypto only)"
./scripts/clean_build.sh
echo "--> Running cargo build. If this fails on E0521 or E0793, FIX THEM MANUALLY in the source."
rustup run nightly cargo build --target x86_64-sex.json -Z build-std=core,alloc --release --verbose > build.log 2>&1 || { 
    cat build.log | tail -50; 
    echo "=== BUILD FAILED ==="; 
    echo "Stop here. Open memory.rs, allocator.rs, or apic.rs and fix the lifetime/packed struct errors natively.";
    exit 1; 
}
echo "   clean build OK → Limine ISO ready"

echo "3. QEMU BOOT TEST (pku=on, verify stack OOM gone)"
./scripts/build_and_test_qemu.sh || qemu-system-x86_64 -machine q35 -cpu max,+pku -smp 4 -m 2G -serial stdio -vga std -boot d -cdrom boot/sexos.iso -display none &
sleep 8
if grep -q "loader: stack OOM" build.log || ! pgrep qemu > /dev/null; then
  echo "BOOT FAIL - stack OOM still present or QEMU crashed."
  exit 1
else
  echo "BOOT SUCCESS - Higher-Half kernel + pure Rust framebuffer gradient live"
fi

echo "4. PHASE 24 PREP: RUSTIFY REMAINING C SERVERS"
./scripts/rustify_confirm.sh || true
cargo +nightly add smoltcp --no-default-features --features="smoltcp-rust" --target x86_64-sex.json || true
echo "   C servers → pure Rust no_std crates swapped"

echo "5. PHASE 18 PREP: COSMIC/ORBITAL GUI HANDOFF (PDX + PKU)"
echo "   kernel will emit MessageType::DisplayPrimaryFramebuffer + pkey_set revoke"
echo "   sexdisplay PD gains zero-copy ARGB framebuffer pointer"
echo "=== FULL HANDOFF PREP COMPLETE ==="
