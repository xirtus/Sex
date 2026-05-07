#!/bin/bash
# sexos-pdx-enforce-boot-v2.sh

set -euo pipefail
echo "✦ SEX MICROKERNEL SASOS — ENFORCING CLEAN PHASE 21 BOOT"

PROJECT_ROOT="/home/xirtus_arch/Documents/microkernel"
cd "$PROJECT_ROOT"

# 1. PURGE
echo "→ Nuking old artifacts..."
cargo clean --quiet
rm -rf iso_root/ *.iso 2>/dev/null || true
mkdir -p iso_root/servers/

# 2. WRITE CFG
echo "→ Writing fresh limine.cfg..."
cat > limine.cfg << 'CFG_EOF'
TIMEOUT=1

:SexOS
    PROTOCOL=limine
    KERNEL_PATH=boot:///sexos-kernel
    MODULE_PATH=boot:///servers/sexdisplay
    VIDEO_MODE=1280x720,32
CFG_EOF

# 3. STAGING
cp limine.cfg iso_root/limine.cfg

# 4. STUBS
echo "→ Injecting stubs..."
for app in apps/cosmic-applets apps/cosmic-edit; do
    [ -d "$app" ] || continue
    mkdir -p "$app/src"
    cat << 'SRC_EOF' > "$app/src/main.rs"
#![no_std]
#![no_main]
extern crate alloc;
use sex_pdx::{pdx_call, PDX_SEX_WINDOW_CREATE, SexWindowCreateParams};
#[no_mangle]
pub extern "C" fn _start() -> ! {
    let title = b"NativeSexOS";
    let params = SexWindowCreateParams { x: 0, y: 0, w: 800, h: 600, title: title };
    let arg0 = (&params as *const SexWindowCreateParams) as u64;
    let _ = unsafe { pdx_call(5, PDX_SEX_WINDOW_CREATE, arg0, 0, 0) };
    loop {}
}
SRC_EOF
done

# 5. BUILD
echo "→ Rebuilding..."
bash build_payload.sh
make iso

# 6. INSPECTION
echo "→ INSPECTING limine.cfg INSIDE THE NEW ISO..."
if command -v 7z >/dev/null 2>&1; then
    7z l sexos-v*.iso | grep -E 'limine.cfg|sexos-kernel|sexdisplay|linitrd' || true
    7z x -so sexos-v*.iso limine.cfg 2>/dev/null || true
fi

echo "→ Booting..."
make run-sasos
