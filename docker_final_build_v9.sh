#!/bin/bash
set -e
echo "=== [SexOS] Patching Linker & Rebuilding ==="

# 1. Rewrite the Linker Script for Higher-Half Memory
cat << 'LD_EOF' > linker.ld
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
        KEEP(*(.requests)) /* Keep Limine requests if they exist */
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
echo "Updated linker.ld for Higher-Half mapping."

# 2. Compile Kernel
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tlinker.ld"
cargo build --release --package sex-kernel \
    --target x86_64-sex.json \
    -Zbuild-std=core,alloc \
    -Zjson-target-spec

# 3. Assemble ISO
mkdir -p build_dir/boot
KERNEL_BIN=$(find target -name "sex-kernel" | grep release | head -n 1)
cp "$KERNEL_BIN" build_dir/boot/sexos-kernel
cp limine.cfg build_dir/
cp limine_bin/limine-bios-cd.bin build_dir/
cp limine_bin/limine-bios.sys build_dir/

xorriso -as mkisofs -b limine-bios-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    build_dir -o sexos-v1.0.0.iso

# 4. Seal
./limine_bin/limine bios-install sexos-v1.0.0.iso

echo "=== SUCCESS: Higher-Half Kernel ISO built! ==="
