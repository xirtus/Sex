#!/bin/bash
set -euo pipefail
echo "✦ SEX MICROKERNEL SASOS — EXECUTING SURGICAL REPAIR"
PROJECT_ROOT="/home/xirtus_arch/Documents/microkernel"
cd "$PROJECT_ROOT"

cargo clean --quiet
rm -rf target/x86_64-sex/ iso_root/ *.iso 2>/dev/null
mkdir -p iso_root/servers/

# Staging
if [ -d "limine" ]; then
    cp limine/limine-bios.sys iso_root/ 2>/dev/null || true
    cp limine/limine-bios-cd.bin iso_root/ 2>/dev/null || true
    cp limine/limine.sys iso_root/ 2>/dev/null || true
fi

# Config
cat > limine.cfg << 'CFG_EOF'
TIMEOUT=1
:SexOS
    PROTOCOL=limine
    KERNEL_PATH=boot:///sexos-kernel
    MODULE_PATH=boot:///servers/sexdisplay
    VIDEO_MODE=1280x720,32
CFG_EOF
cp limine.cfg iso_root/

# Stub
mkdir -p servers/sexdisplay/src
cat << 'SRC_EOF' > servers/sexdisplay/src/main.rs
#![no_std]
#![no_main]
extern crate alloc;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
SRC_EOF

# Build/Run
bash build_payload.sh
make clean
make iso
make run-sasos
