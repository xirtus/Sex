#!/bin/bash
set -e
export PATH="$HOME/.cargo/bin:$PATH"
echo "=== [SexOS] Building inside Docker Container ==="

TARGET_MAIN="kernel/src/main.rs"

# 1. Attributes and Sections are now handled natively in main.rs
# No longer stripping #[used] or .limine_reqs to ensure bootloader compatibility.

# 2. Compile (The container has nightly by default)
export RUSTFLAGS="-A dead_code -A unused_imports -C link-arg=-Tkernel/linker.ld"
echo "--- Building Kernel ---"
cargo +nightly build --release --package sex-kernel \
    --target x86_64-sex.json \
    -Zbuild-std=core,alloc \
    -Zjson-target-spec

echo "--- Building Servers ---"
# Build core servers for the initrd
for server in sexdisplay tuxedo sexc sexfiles sexdrive sexinput sexnet sexnode sexshop sexgemini; do
    echo "Building $server..."
    cargo +nightly build --release --manifest-path servers/$server/Cargo.toml \
        --target x86_64-sex.json \
        -Zbuild-std=core,alloc \
        -Zjson-target-spec
done

# 3. Create initrd.sex
echo "--- Creating initrd.sex ---"
mkdir -p build_initrd
cp target/x86_64-sex/release/sexc \
   target/x86_64-sex/release/sexfiles \
   target/x86_64-sex/release/sexdrive \
   target/x86_64-sex/release/tuxedo \
   target/x86_64-sex/release/sexinput \
   target/x86_64-sex/release/sexnet \
   target/x86_64-sex/release/sexdisplay \
   target/x86_64-sex/release/sexnode \
   target/x86_64-sex/release/sexshop \
   target/x86_64-sex/release/sexgemini \
   build_initrd/

# 4. ISO Assembly
rm -rf build_dir
mkdir -p build_dir/boot
KERNEL_BIN=$(find target -name "sex-kernel" | grep release | head -n 1)
cp "$KERNEL_BIN" build_dir/boot/sexos-kernel

if [ -f "sex-src/bin/sexpac.py" ]; then
    python3 sex-src/bin/sexpac.py --out build_dir/boot/initrd.sex build_initrd/*
else
    cp target/x86_64-sex/release/sexdisplay build_dir/boot/initrd.sex
fi

cp limine.cfg build_dir/

# 5. Sync Limine
if [ ! -d "limine" ]; then
    echo "--- Downloading Limine ---"
    git clone https://github.com/limine-bootloader/limine.git --branch=v7.x-binary --depth=1
fi

if [ ! -f "limine/limine" ]; then
    echo "--- Building Limine ---"
    make -C limine limine
fi
cp limine.cfg build_dir/
cp limine/limine-bios-cd.bin build_dir/
cp limine/limine-uefi-cd.bin build_dir/
cp limine/limine-bios.sys build_dir/

echo "--- Generating Hybrid ISO ---"
xorriso -as mkisofs -b limine-bios-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    --efi-boot limine-uefi-cd.bin \
    -efi-boot-part --efi-boot-image --protective-msdos-label \
    build_dir -o sexos-v1.0.0.iso

./limine/limine bios-install sexos-v1.0.0.iso


cp sexos-v1.0.0.iso dist/

echo "=== SUCCESS: sexos-v1.0.0.iso is ready in Docker ==="
