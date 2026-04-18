#![no_std]
#![no_main]

use sex_pdx::{pdx_call, MessageType};

/// sexinput: Standalone Input Driver (PS/2 + USB HID)
/// Phase 10: Routing HID events to graphical PD.

pub fn sys_park() {
    unsafe {
        core::arch::asm!("syscall", in("rax") 24u64);
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        // 1. Poll PS/2 Scancodes (Simplified for prototype)
        let scancode: u8 = unsafe { x86_64::instructions::port::Port::new(0x60).read() };
        
        if scancode != 0 && scancode != 0xFF {
            // 2. Route scancode to Graphical Server via capability (Slot 5)
            // Using unified HIDEvent message
            let msg = MessageType::HIDEvent { 
                ev_type: 1, // EV_KEY
                code: scancode as u16, 
                value: 1 // Press
            };
            pdx_call(5 /* Display Cap */, 0, &msg as *const _ as u64, 0);
        }

        // 3. FLSCHED wait-free park until HID interrupt
        sys_park();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { sys_park(); }
}
