#!/usr/bin/env bash
set -e
echo "=== SexOS SASOS build payload (Phase 24 — MPK/PKU locked) ==="

rm -rf iso_root 2>/dev/null || true
mkdir -p iso_root/servers iso_root/apps iso_root/boot/limine

cp limine/limine-bios-cd.bin iso_root/boot/limine/
cp limine/limine-uefi-cd.bin iso_root/boot/limine/
cp limine/limine-bios.sys iso_root/boot/limine/

# Kernel
echo "Building sex-kernel ..."
RUSTFLAGS="-C link-arg=-Tkernel/linker.ld" cargo build -Z build-std=core,compiler_builtins,alloc -Zjson-target-spec \
    -Z build-std=core,compiler_builtins,alloc \
    -Z build-std-features=compiler-builtins-mem \
    --package sex-kernel \
    --target x86_64-sex.json \
    --release
cp target/x86_64-sex/release/sex-kernel iso_root/sexos-kernel

# sexdisplay
echo "Building sexdisplay ..."
RUSTFLAGS="-C relocation-model=pic -C link-arg=-pie" cargo build -Z build-std=core,compiler_builtins,alloc -Zjson-target-spec \
    --manifest-path servers/sexdisplay/Cargo.toml \
    --target x86_64-sex.json \
    --release
cp target/x86_64-sex/release/sexdisplay iso_root/servers/sexdisplay

# linen
echo "Building linen ..."
RUSTFLAGS="-C relocation-model=pic -C link-arg=-pie" cargo build -Z build-std=core,compiler_builtins,alloc -Zjson-target-spec \
    --manifest-path apps/linen/Cargo.toml \
    --target x86_64-sex.json \
    --release
cp target/x86_64-sex/release/linen iso_root/apps/linen

# silk-shell
echo "Building silk-shell ..."
RUSTFLAGS="-C relocation-model=pic -C link-arg=-pie" cargo build -Z build-std=core,compiler_builtins,alloc -Zjson-target-spec \
    --manifest-path servers/silk-shell/Cargo.toml \
    --target x86_64-sex.json \
    --release
cp target/x86_64-sex/release/silk-shell iso_root/servers/silk-shell

cp limine.cfg iso_root/ 2>/dev/null || true
echo "✅ All PDX modules staged"
