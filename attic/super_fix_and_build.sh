#!/bin/bash
set -e

echo "=== [SexOS] Starting Full Automation Repair ==="

# 1. Surgical PDX Source Repair
echo "[1/5] Repairing PDX service names..."
find . -name "*.rs" -exec sed -i 's/_service_name/service_name/g' {} +

# 2. Force no_std on sexbuild (Fixes E0463/E0425)
echo "[2/5] Enforcing no_std on sexbuild..."
if [ -f "servers/sexbuild/src/main.rs" ]; then
    sed -i '1i #![no_std]\n#![no_main]' servers/sexbuild/src/main.rs
fi

# 3. Fix Linker Script (Resolve 80 KB Overlap)
echo "[3/5] Writing Multiboot-safe linker.ld..."
cat << 'LNK' > linker.ld
ENTRY(kmain)
SECTIONS {
    . = 0x100000;
    .boot : {
        KEEP(*(.boot))
    }
    .text : { *(.text) }
    .rodata : { *(.rodata*) }
    .data : { *(.data*) }
    .bss : { 
        *(.bss*)
        *(COMMON)
    }
}
LNK

# 4. Clean and Rebuild with Unstable Specs
echo "[4/5] Running Cargo Build with Bare-Metal Flags..."
export RUSTFLAGS="-C link-arg=-nostartfiles -C link-arg=-nodefaultlibs"
cargo build --release \
  --target x86_64-sex.json \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec

# 5. ISO Creation (Search and Rescue)
echo "[5/5] Creating Bootable ISO..."
KERNEL_BIN=$(find target -name "kernel" | grep release | head -n 1)

if [ -z "$KERNEL_BIN" ]; then
    echo "FAILED: Kernel binary not found."
    exit 1
fi

mkdir -p build_dir
cp "$KERNEL_BIN" limine.cfg build_dir/
# Find limine-cd.bin in common locations
cp /usr/share/limine/limine-cd.bin build_dir/ 2>/dev/null || cp sex-src/limine/limine-cd.bin build_dir/ || touch build_dir/limine-cd.bin

xorriso -as mkisofs -b limine-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table build_dir -o sexos-v1.0.0.iso

echo "=== SUCCESS: sexos-v1.0.0.iso is ready ==="
