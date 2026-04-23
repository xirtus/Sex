#!/bin/bash
set -euo pipefail
echo "✦ SEX MICROKERNEL SASOS — EXECUTING SURGICAL DEP-ALIGNMENT"

PROJECT_ROOT="/home/xirtus_arch/Documents/microkernel"
cd "$PROJECT_ROOT"

# 1. CLEAN
cargo clean -p sexdisplay
rm -rf target/x86_64-sex/release/deps/sexdisplay*

# 2. PATCH MANIFEST
cat << 'CFG_EOF' > servers/sexdisplay/Cargo.toml
[package]
name = "sexdisplay"
version = "0.1.0"
edition = "2021"

[dependencies]
sex_pdx = { path = "../../crates/sex-pdx", default-features = false }
sex_graphics = { path = "../../crates/sex-graphics", default-features = false }
alloc = { version = "1.0.0" }

[features]
default = ["no_std"]
no_std = []
CFG_EOF

# 3. SURGICAL FIX
mkdir -p servers/sexdisplay/src
cat << 'LIB_EOF' > servers/sexdisplay/src/lib.rs
#![no_std]
extern crate alloc;

use core::fmt::Write;
use sex_graphics::Rect;

pub struct Compositor {
    pub surface: Rect,
}

impl Compositor {
    pub fn new() -> Self {
        Self {
            surface: Rect { x: 0, y: 0, w: 1280, h: 720 },
        }
    }
}
LIB_EOF

# 4. ENTRY POINT
cat << 'MAIN_EOF' > servers/sexdisplay/src/main.rs
#![no_std]
#![no_main]

use sexdisplay::Compositor;
use sex_pdx::pdx_listen;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let _compositor = Compositor::new();
    loop {
        let _ = pdx_listen();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
MAIN_EOF

# 5. BUILD/RUN
bash build_payload.sh
make clean
make iso
make run-sasos
