#![no_std]
#![no_main]

use libsys::pdx::{pdx_listen, pdx_reply, pdx_call};
use libsys::messages::MessageType;

/// sexinput: Standalone Input Driver (PS/2 + USB HID)
/// Phase 10: Routing HID events to graphical PD.

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        // 1. Poll PS/2 Scancodes (Simplified for prototype)
        let scancode: u8 = unsafe { x86_64::instructions::port::Port::new(0x60).read() };
        
        if scancode != 0 && scancode != 0xFF {
            // 2. Route scancode to Graphical Server (Fixed PD 500) via PDX
            // Using zero-copy event message
            let msg = MessageType::HIDEvent { 
                ev_type: 1, // EV_KEY
                code: scancode as u16, 
                value: 1 // Press
            };
            pdx_call(500, 0, &msg as *const _ as u64, 0);
        }

        // 3. FLSCHED wait-free park until HID interrupt
        unsafe { core::arch::asm!("syscall", in("rax") 24 /* SYS_PARK */); }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("syscall", in("rax") 24); } }
}
