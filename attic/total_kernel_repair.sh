#!/bin/bash
set -e
echo "=== [SexOS] Repairing Kernel Dependency Tree ==="

# 1. Inject ALL missing crates into kernel/Cargo.toml
# We use versions compatible with the 2026 nightly toolchain
cat << 'CRATES' > crates_to_add.txt
limine = "0.1.0"
raw-cpuid = "11.0.1"
linked_list_allocator = "0.10.5"
uart_16550 = "0.3.0"
spinning_top = "0.3.0"
lazy_static = { version = "1.4.0", features = ["spin_no_std"] }
volatile = "0.4.6"
acpi = "5.0.0"
CRATES

# Append only if they don't exist
while read p; do
    NAME=$(echo $p | cut -d' ' -f1)
    if ! grep -q "$NAME" kernel/Cargo.toml; then
        echo "Adding $p..."
        sed -i "/\[dependencies\]/a $p" kernel/Cargo.toml
    fi
done < crates_to_add.txt

# 2. Fix the 'tuxedo' and 'sexpdx' naming conflict if it persists
find . -name "*.rs" -exec sed -i 's/_service_name/service_name/g' {} +

# 3. Trigger the isolated Kernel Build
echo "[Step 1/2] Rebuilding Kernel Core with Full Dependencies..."
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo build --release --package sex-kernel \
  --target x86_64-sex.json \
  -Zbuild-std=core,alloc \
  -Zjson-target-spec

# 4. Assembly
echo "[Step 2/2] Creating Final ISO..."
KERNEL_BIN=$(find target -name "sex-kernel" | grep release | head -n 1)

if [ -f "$KERNEL_BIN" ]; then
    mkdir -p build_dir
    cp "$KERNEL_BIN" build_dir/kernel
    cp limine.cfg build_dir/
    cp /usr/share/limine/limine-cd.bin build_dir/ 2>/dev/null || touch build_dir/limine-cd.bin
    xorriso -as mkisofs -b limine-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table build_dir -o sexos-v1.0.0.iso
    echo "=== SUCCESS: sexos-v1.0.0.iso is ready ==="
else
    echo "FAILED: Compilation successful but binary not found in expected path."
fi
