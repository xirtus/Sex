#![feature(alloc_error_handler)]
extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::boxed::Box;
#![no_std]
#![no_main]

use sex_pdx::{pdx_listen, pdx_reply, Message, MessageType};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        let req = pdx_listen(0);
        let msg = Message::from_u64(req.arg0);
        
        // Demand paging logic (Simulation: success)
        if let MessageType::PageFault { .. } = msg.msg_type() {
            pdx_reply(req.caller_pd, 0);
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
