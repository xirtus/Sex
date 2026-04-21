#!/bin/bash
# =================================================================
# SexOS Phase 19: Substrate Hardening (Structural Restoration)
# =================================================================
set -euo pipefail

echo "─── Step 1: Restoring arch/mod.rs ───"
mkdir -p kernel/src/arch/x86_64
cat > kernel/src/arch.rs << 'ARCH_MOD_EOF'
pub mod x86_64;
ARCH_MOD_EOF

cat > kernel/src/arch/x86_64/mod.rs << 'X86_MOD_EOF'
pub mod serial;
X86_MOD_EOF

echo "─── Step 2: Implementing COM1 Serial Driver ───"
cat > kernel/src/arch/x86_64/serial.rs << 'SERIAL_EOF'
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

echo "─── Step 3: Integrating Unitary Root (lib.rs) ───"
cat > kernel/src/lib.rs << 'LIB_EOF'
#![no_std]
#![feature(abi_x86_interrupt)]

pub mod arch;
pub mod memory;

use limine::request::{FramebufferRequest, HhdmRequest, MemoryMapRequest};

#[used]
pub static FB_REQUEST: FramebufferRequest = FramebufferRequest::new(0);
#[used]
pub static Mmap_REQUEST: MemoryMapRequest = MemoryMapRequest::new(0);
#[used]
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new(0);

// Aliases for allocator compatibility
pub static MEMMAP_REQUEST: &MemoryMapRequest = &Mmap_REQUEST;

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

echo "─── Step 4: Atomic Synthesis (RUSTC_BOOTSTRAP Force) ───"
RUSTC_BOOTSTRAP=1 cargo build \
    --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -p sex-kernel \
    --release

echo "✅ SUBSTRATE HARDENING COMPLETE."
echo "Launch QEMU with: -cpu max,+pku -serial stdio"
