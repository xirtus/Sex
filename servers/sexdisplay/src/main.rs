#![no_std]
#![no_main]

use libsys::pdx::{pdx_listen, pdx_reply};
use libsys::messages::MessageType;
use libsys::sched::park_on_ring;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        park_on_ring();
        let req = pdx_listen(0);
        let msg = unsafe { *(req.arg0 as *const MessageType) };

        match msg {
            MessageType::GpuCall { .. } => {
                pdx_reply(req.caller_pd, 0);
            },
            MessageType::HIDEvent { .. } => {
                pdx_reply(req.caller_pd, 0);
            },
            _ => pdx_reply(req.caller_pd, u64::MAX),
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("syscall", in("rax") 24); } }
}
