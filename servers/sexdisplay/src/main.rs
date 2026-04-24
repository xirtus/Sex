#![no_std]
#![no_main]

use sex_pdx::{pdx_listen, pdx_reply, sys_yield, serial_println};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("[sexdisplay] PD1 start — waiting for FB handoff");

    let msg = pdx_listen();

    if msg.type_id == 0x11 {
        let fb_ptr = msg.arg0 as *mut u32;
        let width  = (msg.arg1 & 0xFFFF_FFFF) as u32;
        let height = (msg.arg1 >> 32) as u32;
        serial_println!("[sexdisplay] FB received {}x{} @ {:#x}", width, height, msg.arg0);

        // Red probe: confirms PKEY 1 framebuffer write path
        unsafe {
            for i in 0..(width * height) as usize {
                fb_ptr.add(i).write_volatile(0xFFFF0000);
            }
        }
        serial_println!("[sexdisplay] Red fill complete — PKEY 1 write OK");

        loop {
            let cmd = pdx_listen();
            serial_println!("[sexdisplay] msg type_id={:#x} from PD {}", cmd.type_id, cmd.caller_pd);
            match cmd.type_id {
                0xDE => { // OP_WINDOW_CREATE
                    serial_println!("[sexdisplay] OP_WINDOW_CREATE -> ack PD {}", cmd.caller_pd);
                    pdx_reply(cmd.caller_pd as u64);
                }
                0xDD => { // OP_COMPOSITOR_COMMIT
                }
                _ => sys_yield(),
            }
        }
    }
    serial_println!("[sexdisplay] ERROR: no FB message (type_id={:#x})", msg.type_id);
    loop { sys_yield(); }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { sys_yield(); }
}
