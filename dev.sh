#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

TARGET=x86_64-sex.json

build() {
RUSTFLAGS="-C relocation-model=pic -C link-arg=-pie" cargo build \
  -p sexdisplay -p silkbar \
  --target "$TARGET" \
  -Z build-std=core,compiler_builtins,alloc \
  -Z build-std-features=compiler-builtins-mem \
  --release
}

stage_bins() {
cp -f target/x86_64-sex/release/sexdisplay iso_root/servers/sexdisplay
cp -f target/x86_64-sex/release/silkbar    iso_root/servers/silkbar
}

iso() {
rm -f sexos-v1.0.0.iso
xorriso -as mkisofs -R -r -J \
-b boot/limine/limine-bios-cd.bin \
-no-emul-boot -boot-load-size 4 -boot-info-table \
--efi-boot boot/limine/limine-uefi-cd.bin \
-efi-boot-part --efi-boot-image \
--protective-msdos-label \
iso_root -o sexos-v1.0.0.iso
}

run() {
qemu-system-x86_64 \
-machine q35 \
-cpu max,pku=on \
-smp 4 \
-m 2G \
-vga std \
-serial stdio \
-boot d \
-cdrom "$(pwd)/sexos-v1.0.0.iso"
}

case "${1:-all}" in
build) build ;;
iso) build; stage_bins; iso ;;
run) run ;;
all) build; stage_bins; iso; run ;;
esac
