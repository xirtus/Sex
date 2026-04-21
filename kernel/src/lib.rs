#![no_std]
#![feature(abi_x86_interrupt)]

pub mod arch;
pub mod memory;
pub const MAP_MEMORY_SYSCALL_NUM: u64 = 30;
pub const ALLOCATE_MEMORY_SYSCALL_NUM: u64 = 31;

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

            // 3. Trigger Hardware Enforcement (PKU Lockdown)
            serial_println!("[SexOS] Deploying Page Table Walker...");
            memory::pku::init_pku_isolation();
        }
    }
}
