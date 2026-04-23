#!/bin/bash
set -euo pipefail
echo "✦ SEX MICROKERNEL SASOS — RESTORING PHASE 21 STABILITY"

PROJECT_ROOT="/home/xirtus_arch/Documents/microkernel"
cd "$PROJECT_ROOT"

# 1. PURGE
cargo clean --quiet
rm -rf iso_root/ *.iso 2>/dev/null || true
mkdir -p iso_root/servers/

# 2. CFG
cat > limine.cfg << 'CFG_EOF'
TIMEOUT=1
:SexOS
    PROTOCOL=limine
    KERNEL_PATH=boot:///sexos-kernel
    MODULE_PATH=boot:///servers/sexdisplay
    VIDEO_MODE=1280x720,32
CFG_EOF
cp limine.cfg iso_root/

# 3. KERNEL CLEANUP
sed -i '/static.*HhdmRequest/d' kernel/src/init.rs || true
sed -i '/static.*MemmapRequest/d' kernel/src/init.rs || true
sed -i '/static.*FramebufferRequest/d' kernel/src/init.rs || true
sed -i '/static.*BaseRevision/d' kernel/src/init.rs || true

# 4. SERVER STUB
mkdir -p servers/sexdisplay/src
cat << 'SRC_EOF' > servers/sexdisplay/src/main.rs
#![no_std]
#![no_main]
extern crate alloc;
use sex_pdx::{pdx_listen};
#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop { let _ = pdx_listen(); }
}
SRC_EOF

# 5. APPLET STUBS
for app in apps/cosmic-applets apps/cosmic-edit; do
    mkdir -p "$app/src"
    cat << 'SRC_EOF' > "$app/src/main.rs"
#![no_std]
#![no_main]
extern crate alloc;
use sex_pdx::{pdx_call, PDX_SEX_WINDOW_CREATE, SexWindowCreateParams};
#[no_mangle]
pub extern "C" fn _start() -> ! {
    let params = SexWindowCreateParams { w: 1280, h: 720, title: [0; 64] };
    let _ = unsafe { pdx_call(5, PDX_SEX_WINDOW_CREATE, &params as *const _ as usize, 0, 0) };
    loop {}
}
SRC_EOF
done

# 6. BUILD
bash build_payload.sh
make clean
make iso
make run-sasos
