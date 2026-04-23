#![no_std]
#![no_main]

use sex_pdx::*;
use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

static mut REGISTRY: [([u8; 32], u32); 16] = [([0; 32], 0); 16];
static mut REG_COUNT: usize = 0;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut name = [0u8; 32];
    name[..7].copy_from_slice(b"sexshop");
    unsafe { pdx_call(0, PDX_DISCOVER_SERVICE as u64, name.as_ptr() as u64, 0, 0) };

    loop {
        let event = unsafe { pdx_listen(0) };

        if event.num == 0 {
            unsafe { sys_yield(); }
        } else if event.num == PDX_DISCOVER_SERVICE as u64 {
            // Register logic
            let name_ptr = event.arg0 as *const u8;
            unsafe {
                if REG_COUNT < 16 {
                    let mut name = [0u8; 32];
                    core::ptr::copy_nonoverlapping(name_ptr, name.as_mut_ptr(), 32);
                    REGISTRY[REG_COUNT] = (name, event.caller_pd);
                    REG_COUNT += 1;
                }
            }
        } else if event.num == 0x101 {
            // Lookup logic (0x101 is discovery lookup)
            let name_ptr = event.arg0 as *const [u8; 32];
            let mut found = 0;
            unsafe {
                let name = *name_ptr;
                for i in 0..REG_COUNT {
                    if REGISTRY[i].0 == name {
                        found = REGISTRY[i].1;
                        break;
                    }
                }
            }
            unsafe { pdx_reply(event.caller_pd, found as u64) };
        }
    }
}
