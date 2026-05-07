#!/bin/bash
set -e
echo "=== [SexOS] The Cache-Busting Higher-Half Build ==="

# 1. FORCE CARGO TO DELETE THE OLD KERNEL
echo "Clearing Cargo cache..."
cargo clean

# 2. Find and overwrite the linker script wherever it lives
LINKER_FILES=$(find . -name "linker.ld")
if [ -z "$LINKER_FILES" ]; then
    echo "Creating new linker.ld"
    LINKER_FILES="./linker.ld"
fi

for lf in $LINKER_FILES; do
    cat << 'LD_EOF' > "$lf"
OUTPUT_FORMAT(elf64-x86-64)
ENTRY(_start)

SECTIONS
{
    /* THIS IS THE FIX: Move base address to the Higher Half */
    . = 0xffffffff80200000;

    .text : ALIGN(4K) {
        *(.text .text.*)
    }

    .rodata : ALIGN(4K) {
        *(.rodata .rodata.*)
    }

    .data : ALIGN(4K) {
        *(.data .data.*)
        KEEP(*(.requests_start))
        KEEP(*(.requests))
        KEEP(*(.requests_end))
    }

    .bss : ALIGN(4K) {
        *(COMMON)
        *(.bss .bss.*)
    }

    /DISCARD/ : {
        *(.eh_frame)
        *(.note .note.*)
    }
}
LD_EOF
    echo "Patched $lf for Higher-Half Memory."
done

# 3. Build FRESH (This should take longer than 0.59 seconds!)
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo build --release --package sex-kernel \
    --target x86_64-sex.json \
    -Zbuild-std=core,alloc \
    -Zjson-target-spec

# 4. Assemble ISO
mkdir -p build_dir/boot
KERNEL_BIN=$(find target -name "sex-kernel" | grep release | head -n 1)
cp "$KERNEL_BIN" build_dir/boot/sexos-kernel
cp limine.cfg build_dir/
cp limine_bin/limine-bios-cd.bin build_dir/
cp limine_bin/limine-bios.sys build_dir/

xorriso -as mkisofs -b limine-bios-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    build_dir -o sexos-v1.0.0.iso

./limine_bin/limine bios-install sexos-v1.0.0.iso

echo "=== SUCCESS: Cache cleared, Higher-Half Kernel built! ==="
