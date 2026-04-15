#![no_std]
#![no_main]

use libsys::pdx::{pdx_listen, pdx_reply, pdx_call};
use libsys::messages::MessageType;
use libsys::sched::park_on_ring;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        // Poll for interrupts or messages
        park_on_ring();

        let req = pdx_listen(0);
        let msg = unsafe { *(req.arg0 as *const MessageType) };

        match msg {
            MessageType::NetCall { .. } => {
                let reply = MessageType::NetReply { status: 0, size: 0, socket_cap: 10 };
                pdx_reply(req.caller_pd, &reply as *const _ as u64);
            },
            _ => pdx_reply(req.caller_pd, u64::MAX),
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { park_on_ring(); }
}
