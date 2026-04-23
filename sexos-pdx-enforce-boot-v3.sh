#!/bin/bash
set -euo pipefail
echo "✦ SEX MICROKERNEL SASOS — TOTAL LIMINE CFG PURGE"

PROJECT_ROOT="/home/xirtus_arch/Documents/microkernel"
cd "$PROJECT_ROOT"

# 1. PURGE
cargo clean --quiet
find . -name "limine.cfg" -exec rm -f {} +
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

# 3. LOCATE
mkdir -p iso_root/boot/limine
mkdir -p iso_root/limine
cp limine.cfg iso_root/
cp limine.cfg iso_root/boot/limine.cfg
cp limine.cfg iso_root/boot/limine/limine.cfg
cp limine.cfg iso_root/limine/limine.cfg
mkdir -p limine
cp limine.cfg limine/limine.cfg

# 4. STUBS
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
bash build_payload.sh
rm -f *.iso
make iso

# 6. AUDIT
echo "--- AUDIT ---"
if command -v 7z >/dev/null 2>&1; then
    7z l sexos-v*.iso | grep -E 'limine.cfg|sexos-kernel|sexdisplay|linitrd'
    7z x -so sexos-v*.iso limine.cfg 2>/dev/null || true
fi

make run-sasos
