#!/bin/bash
set -e

# 1. Build the binary with the fix
echo "--- 1. Synthesizing Fixed Kernel ---"
docker run --platform linux/amd64 --rm -v $(pwd):/sex -w /sex \
-e CARGO_UNSTABLE_JSON_TARGET_SPEC=true \
-e CARGO_UNSTABLE_BUILD_STD=core,alloc \
-e CARGO_UNSTABLE_BUILD_STD_FEATURES=compiler-builtins-mem \
--entrypoint /bin/bash sexos-builder:v28 -c "
    rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu && \
    cargo +nightly build --package sex-kernel --target x86_64-sex.json --release \
    --config \"target.x86_64-sex.rustflags=['-C', 'linker=rust-lld', '-C', 'target-cpu=skylake', '-C', 'link-arg=--script=kernel/linker.ld', '-C', 'code-model=kernel', '-C', 'relocation-model=static']\"
"

# 2. Package ISO
echo "--- 2. Packaging SASOS ISO ---"
mkdir -p iso_root
cp target/x86_64-sex/release/sex-kernel iso_root/sexos-kernel
cp limine.cfg limine-bios.sys limine-bios-cd.bin limine-uefi-cd.bin iso_root/ 2>/dev/null || true

xorriso -as mkisofs -b limine-bios-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --efi-boot limine-uefi-cd.bin \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        iso_root -o sexos-v1.0.0.iso

# 3. Launch with Triple-Fault Trap (-no-reboot)
echo "--- 3. Launching (Trap Active) ---"
qemu-system-x86_64 -cdrom sexos-v1.0.0.iso \
                   -serial stdio \
                   -m 512M \
                   -vga std \
                   -cpu max,+pku \
                   -no-reboot \
                   -d int,cpu_reset
