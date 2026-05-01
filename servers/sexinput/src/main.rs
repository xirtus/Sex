#![no_std]
#![no_main]

use sex_pdx::*;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop { sys_yield(); }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    sex_rt::heap_init();
    serial_println!("[sexinput] Normalizer Starting...");

    unsafe {
        sys_set_state(SVC_STATE_LISTENING);
    }

    loop {
        // 1. Listen for raw scancodes from kernel (SLOT_INPUT = 3)
        let req = pdx_listen_raw(SLOT_INPUT);

        // Kernel RawInput is type 0x201, arg0 = scancode
        if req.type_id == 0x201 {
            let scancode = req.arg0;
            // serial_println!("[sexinput] Raw scancode: {:#x}", scancode);

            // 2. Normalize and forward to silk-shell (SLOT_SHELL = 6)
            // Typed event via 0x202: arg0=code(break-bit stripped), arg1=1(press)/0(release), arg2=EV_KEY
            let value = if scancode & 0x80 == 0 { 1 } else { 0 };
            let code = (scancode & 0x7F) as u64;
            pdx_call(SLOT_SHELL, 0x202, code, value, EV_KEY);
        } else {
            sys_yield();
        }
    }
}
