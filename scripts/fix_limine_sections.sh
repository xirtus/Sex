#!/bin/bash
set -e

echo "--- 1. Patching main.rs for Section Mutability ---"

# We must ensure the requests are in a section Limine can write to (.limine_reqs)
# and that they are 8-byte aligned as per the protocol.

cat > kernel/src/main.rs << 'RS_EOF'
#![no_std]
#![no_main]

use limine::request::{FramebufferRequest, HhdmRequest, MemmapRequest, RsdpRequest};
use sex_kernel;

// Force requests into the .limine_reqs section and ensure 8-byte alignment
#[used]
#[link_section(".limine_reqs")]
static FB_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[link_section(".limine_reqs")]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[used]
#[link_section(".limine_reqs")]
static MEM_REQUEST: MemmapRequest = MemmapRequest::new();

#[used]
#[link_section(".limine_reqs")]
static RSDP_REQUEST: RsdpRequest = RsdpRequest::new();

#[no_mangle]
pub extern "C" fn _start() -> ! {
    sex_kernel::serial_println!("[SexOS] Boot Handshake Successful.");

    let hhdm = HHDM_REQUEST.response().expect("hhdm failed");
    let fb_res = FB_REQUEST.response().expect("fb failed");
    
    let fb = fb_res.framebuffers().iter().next().expect("no framebuffer");
    
    // Safety: We are mapping the physical address provided by Limine 
    // to a virtual address using the HHDM offset.
    let fb_ptr = (fb.address() as u64) as *mut u32;

    sex_kernel::serial_println!("Sex: Framebuffer found at {:?}, {}x{} (pitch={})", 
        fb.address(), fb.width, fb.height, fb.pitch);

    // Draw the Blue Gradient test pattern
    for y in 0..fb.height {
        for x in 0..fb.width {
            let color = (x as u32 % 255) | ((y as u32 % 255) << 8) | (0xFF << 16);
            let index = (y * (fb.pitch / 4) + x) as usize;
            unsafe {
                fb_ptr.add(index).write_volatile(color);
            }
        }
    }

    sex_kernel::serial_println!("Sex: Pure Rust framebuffer test pattern drawn.");

    loop {
        x86_64::instructions::hlt();
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    sex_kernel::serial_println!("KERNEL PANIC: {}", info);
    loop {
        x86_64::instructions::hlt();
    }
}
RS_EOF

echo "--- 2. Rebuilding Kernel ---"
./scripts/fix_kernel_api.sh

echo "--- 3. Launching with Debug Logging ---"
# We add -d int,cpu_reset to see if the kernel is triple-faulting
./scripts/launch_sasos_v4.sh
