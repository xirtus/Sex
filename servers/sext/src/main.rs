#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // In Phase 19, this will be replaced by a PDX call to the kernel 
    // to map the shared heap region.
    loop {}
}
extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::boxed::Box;

use sex_pdx::{pdx_listen_raw, pdx_reply, Message, MessageType};

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        let req = pdx_listen_raw(0);
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
