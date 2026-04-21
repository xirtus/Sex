#!/bin/bash
# 🛠️ SASOS Phase 20.5: Ambiguity Removal & Synthesis
set -euo pipefail

echo "─── Step 1: Removing Module Ambiguity ───"
# Delete the old flat file to favor the memory/ directory structure
rm -f kernel/src/memory.rs
echo "✅ Ambiguity resolved: memory.rs removed in favor of memory/mod.rs"

echo "─── Step 2: Ensuring Module Tree Integrity ───"
mkdir -p kernel/src/memory
cat > kernel/src/memory/mod.rs << 'MEM_MOD_EOF'
pub mod allocator;
pub mod pku;
MEM_MOD_EOF

echo "─── Step 3: Atomic Synthesis (The macOS Killshot) ───"
# Re-enforcing the Synthesis Trinity
export RUSTFLAGS="-C linker=lld"
export RUSTC_BOOTSTRAP=1

rustup run nightly cargo build \
    --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z json-target-spec \
    -p sex-kernel \
    --release

echo "✅ PHASE 20.5 SYNTHESIS SUCCESSFUL."
echo "1. Run ./scripts/final_payload.sh to mint the ISO."
echo "2. Launch QEMU: qemu-system-x86_64 -cdrom sexos-sasos.iso -serial stdio -m 512 -cpu max,+pku"
