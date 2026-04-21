#!/bin/bash
# =================================================================
# SexOS Phase 19: Substrate Hardening (LLD Linker Override)
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

echo "─── Step 2: Implementing Serial Driver ───"
cat > kernel/src/arch/x86_64/serial.rs << 'SERIAL_EOF'
use core::fmt;
use x86::io::{outb, inb};

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
            while (inb(self.port + 5) & 0x20) == 0 {}
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

use limine::request::{FramebufferRequest, HhdmRequest, MemmapRequest};

#[used]
pub static FB_REQUEST: FramebufferRequest = FramebufferRequest::new();
#[used]
pub static MEMMAP_REQUEST: MemmapRequest = MemmapRequest::new();
#[used]
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[allow(static_mut_refs)]
pub fn kernel_init() {
    // 1. Initialize Telemetry
    unsafe { arch::x86_64::serial::COM1.init(); }
    serial_println!("[SexOS] Substrate Phase 19 Hardened.");

    // 2. Claim Framebuffer and clear to Midnight Blue
    if let Some(fb_res) = FB_REQUEST.response() {
        if let Some(fb) = fb_res.framebuffers().first() {
            let ptr = fb.address() as *mut u32;
            serial_println!("[SexOS] Claiming FB: {}x{} at Midnight Blue", fb.width, fb.height);
            
            let size = (fb.width * fb.height) as isize;
            for i in 0..size {
                unsafe { *ptr.offset(i) = 0x191970; }
            }
        }
    }
}
LIB_EOF

echo "─── Step 4: Updating main.rs Trampoline ───"
cat > kernel/src/main.rs << 'MAIN_EOF'
#![no_std]
#![no_main]

use sex_kernel::kernel_init;

#[no_mangle]
extern "C" fn _start() -> ! {
    kernel_init();
    loop { core::hint::spin_loop(); }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
MAIN_EOF

echo "─── Step 5: Atomic Synthesis (Linker Override) ───"
# We inject the linker override via RUSTFLAGS
export RUSTFLAGS="-C linker=lld"
export RUSTC_BOOTSTRAP=1

cargo build \
    --target x86_64-sex.json \
    -Z build-std=core,alloc \
    -p sex-kernel \
    --release

echo "✅ SYSTEM SYNTHESIS SUCCESSFUL."
echo "1. Run ./scripts/final_payload.sh to mint the ISO."
echo "2. Launch QEMU: qemu-system-x86_64 -cdrom sexos-sasos.iso -serial stdio -m 512 -cpu max,+pku"
