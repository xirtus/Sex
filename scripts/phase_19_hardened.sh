#!/bin/bash
# =================================================================
# SexOS Phase 19: Substrate Hardening (JSON Spec Fix + Nightly)
# =================================================================
set -euo pipefail

PROJECT_ROOT=$(pwd)
ARCH_DIR="kernel/src/arch/x86_64"
MEM_DIR="kernel/src/memory"

echo "─── Step 0: Validating Nightly Environment ───"
rustup component add rust-src --toolchain nightly

echo "─── Step 1: Implementing COM1 Serial Debug Bridge ───"
mkdir -p "$ARCH_DIR"
cat > "$ARCH_DIR/serial.rs" << 'SERIAL_EOF'
use core::fmt;
use x86::io::outb;

pub struct SerialPort { port: u16 }

impl SerialPort {
    pub const fn new(port: u16) -> Self { Self { port } }
    pub fn init(&self) {
        unsafe {
            outb(self.port + 1, 0x00);
            outb(self.port + 3, 0x80);
            outb(self.port + 0, 0x03);
            outb(self.port + 1, 0x00);
            outb(self.port + 3, 0x03);
            outb(self.port + 2, 0xC7);
            outb(self.port + 4, 0x0B);
        }
    }
    pub fn send(&self, data: u8) {
        unsafe {
            while (x86::io::inb(self.port + 5) & 0x20) == 0 {}
            outb(self.port, data);
        }
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() { self.send(byte); }
        Ok(())
    }
}

pub static mut COM1: SerialPort = SerialPort::new(0x3F8);

#[macro_export]
macro_rules! serial_println {
    ($($arg:tt)*) => {
        unsafe {
            use core::fmt::Write;
            let _ = writeln!($crate::arch::x86_64::serial::COM1, $($arg)*);
        }
    };
}
SERIAL_EOF

echo "─── Step 2: Implementing Intel PKU (MPK) Protections ───"
cat > "$MEM_DIR/pku.rs" << 'PKU_EOF'
pub unsafe fn wrpkru(pkru: u32) {
    let eax = pkru;
    let ecx = 0;
    let edx = 0;
    core::arch::asm!("wrpkru", in("eax") eax, in("ecx") ecx, in("edx") edx);
}

pub unsafe fn rdpkru() -> u32 {
    let (eax, edx): (u32, u32);
    core::arch::asm!("rdpkru", out("eax") eax, out("edx") edx, in("ecx") 0);
    eax
}

pub fn set_pkey_perm(pkey: u8, perm: u8) {
    unsafe {
        let mut pkru = rdpkru();
        pkru &= !(0b11 << (pkey * 2));
        pkru |= (perm as u32) << (pkey * 2);
        wrpkru(pkru);
    }
}
PKU_EOF

echo "─── Step 3: Claiming the Midnight Blue Framebuffer ───"
cat > kernel/src/lib.rs << 'LIB_EOF'
#![no_std]
#![feature(abi_x86_interrupt)]

pub mod arch;
pub mod memory;

use limine::FramebufferRequest;

static FB_REQUEST: FramebufferRequest = FramebufferRequest::new(0);

pub fn kernel_init() {
    unsafe { arch::x86_64::serial::COM1.init(); }
    serial_println!("[SexOS] Substrate Phase 19 Hardening Active.");

    if let Some(fb_res) = FB_REQUEST.get_response().get() {
        let fb = &fb_res.framebuffers()[0];
        let ptr = fb.address.as_ptr().unwrap() as *mut u32;
        
        serial_println!("[SexOS] Claiming FB: {}x{} at Midnight Blue", fb.width, fb.height);
        
        for i in 0..(fb.width * fb.height) as isize {
            unsafe { *ptr.offset(i) = 0x191970; }
        }
    }
}
LIB_EOF

echo "─── Step 4: Nightly System Synthesis ───"
rustup run nightly cargo build \
    --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -Z json-target-spec \
    -p sex-kernel \
    --release

echo "✅ SUBSTRATE HARDENING COMPLETE."
echo "Launch QEMU with: -cpu max,+pku -serial stdio"
