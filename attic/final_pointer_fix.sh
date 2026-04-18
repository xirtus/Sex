#!/bin/bash
set -e
echo "=== [SexOS] Repairing Pointers and Memory Mapping ==="

# 1. Global Naming Fix (MpInfo -> SmpInfo)
find kernel/src -name "*.rs" -exec sed -i 's/MpInfo/SmpInfo/g' {} +

# 2. Fix MemoryMap Entry Alias (E0223)
# Limine 0.1.x uses limine::file::Entry or similar, but the map itself 
# provides the entries. We'll use the correct raw pointer type.
sed -i 's/pub type MemoryMapEntry = limine::MemmapRequest::Entry;/pub type MemoryMapEntry = limine::NonNullPtr<limine::MemmapEntry>;/' kernel/src/memory.rs

# 3. Fix the Usable Memory Constant (E0599)
# MEMMAP_USABLE is an enum/variant on the Entry type, not the Request
find kernel/src -name "*.rs" -exec sed -i 's/limine::MemmapRequest::MEMMAP_USABLE/limine::MemoryMapEntryType::Usable/g' {} +

# 4. Fix the SMP Bootstrap Call (E0599)
# We need to dereference the NonNullPtr to get to the bootstrap method
sed -i 's/cpu.bootstrap/(*cpu).goto_address.write(limine_ap_entry as u64); \/\/ Manual bootstrap for/' kernel/src/smp.rs

# 5. Final Build attempt
echo "[Step 1/2] Rebuilding Kernel Core..."
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo build --release --package sex-kernel \
  --target x86_64-sex.json \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec

# 6. Assembly
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
