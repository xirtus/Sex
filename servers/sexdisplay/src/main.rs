#![no_std]
#![no_main]

use sex_pdx::{pdx_listen, pdx_reply, sys_yield, MessageType};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Wait for the Kernel to hand over the hardware framebuffer PKEY 1 capability
    let (_kernel_pd, msg) = pdx_listen();

    if let MessageType::DisplayPrimaryFramebuffer { virt_addr, width, height, .. } = msg {
        let fb_ptr = virt_addr as *mut u32;

        // EXECUTE: Cosmic Purple Flip (0xFF663399)
        unsafe {
            for i in 0..(width * height) as usize {
                fb_ptr.add(i).write_volatile(0xFF663399);
            }
        }

        // Enter high-frequency blit loop
        loop {
            let (caller_pd, cmd) = pdx_listen();
            match cmd {
                MessageType::WindowCreate => {
                    // Hazard 2 Fix: Return the SAS-safe Shared Canvas address
                    pdx_reply(caller_pd);
                }
                MessageType::CompositorCommit => {
                    // Blit Shared Canvas -> Hardware FB (Phase 26)
                }
                _ => sys_yield(),
            }
        }
    }
    loop { sys_yield(); }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { sys_yield(); }
}
