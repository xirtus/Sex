#![no_std]
#![no_main]

use limine::request::{FramebufferRequest, HhdmRequest, MemmapRequest};
use sex_kernel;

#[used]
#[link_section = ".limine_reqs"]
static FB_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[link_section = ".limine_reqs"]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[used]
#[link_section = ".limine_reqs"]
static MEM_REQUEST: MemmapRequest = MemmapRequest::new();

#[no_mangle]
pub extern "C" fn _start() -> ! {
    sex_kernel::serial_println!("[SexOS] Phase 18: Testing Hardware Bridge...");

    let hhdm = HHDM_REQUEST.response().expect("hhdm failed");
    let fb_res = FB_REQUEST.response().expect("fb failed");
    let fb = fb_res.framebuffers().iter().next().expect("no FB");
    
    let raw_addr = fb.address() as u64;
    
    // RESILIENCE LOGIC: 
    // If address is < hhdm.offset, it is a Physical Address (needs shift).
    // If address is >= hhdm.offset, it is already a Virtual Address (use as-is).
    let fb_virt_addr = if raw_addr < hhdm.offset {
        raw_addr + hhdm.offset
    } else {
        raw_addr
    };

    let fb_ptr = fb_virt_addr as *mut u32;

    sex_kernel::serial_println!("Sex: Bridge Open at Virtual {:p}", fb_ptr);

    // Draw the Blue Gradient test pattern
    for y in 0..fb.height {
        for x in 0..fb.width {
            let color = (x as u32 % 255) | ((y as u32 % 255) << 8) | (0xFF << 16);
            let index = (y * (fb.pitch / 4) + x) as usize;
            unsafe {
                // write_volatile ensures the compiler doesn't "optimize away" the pixels
                fb_ptr.add(index).write_volatile(color);
            }
        }
    }

    sex_kernel::serial_println!("[SexOS] SUCCESS: Hardware Bridge Verified.");

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
