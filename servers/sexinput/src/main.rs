#![no_std]
#![no_main]

use sex_pdx::*;
use sex_pdx::sys_yield;
use core::panic::PanicInfo;
use core::arch::asm;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Register with sexshop (PD 0, discovery service)
    let mut name = [0u8; 32];
    name[..14].copy_from_slice(b"io.sexos.input");
    unsafe { 
        pdx_call(0, PDX_DISCOVER_SERVICE as u64, name.as_ptr() as u64, 0, 0);
    }

    loop {
        // 2. Poll PS/2 status port (0x64)
        let status: u8 = unsafe { 
            let mut val: u8;
            asm!("in al, dx", in("dx") 0x64u16, out("al") val);
            val
        };

        // If buffer full (bit 0), read data port (0x60)
        if (status & 1) != 0 {
            let scancode: u8 = unsafe {
                let mut val: u8;
                asm!("in al, dx", in("dx") 0x60u16, out("al") val);
                val
            };

            // 3. Broadcast to compositor/focus (Slot 1 = sexdisplay)
            // PDX_INPUT_EVENT definition missing in lib.rs, assuming 0x22
            let msg = PdxMessage {
                msg_type: MessageType::HIDEvent { 
                    ev_type: 1, 
                    code: scancode as u16, 
                    value: 1 
                },
                payload: [0u8; 64],
            };
            unsafe { 
                pdx_call(1, 0x22, &msg as *const _ as u64, 0, 0); 
            }
        } else {
            unsafe { sys_yield(); }
        }
    }
}
