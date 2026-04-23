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
extern crate spin;
extern crate sex_rt;

mod pdx;

use crate::pdx::handle_ld_message;
use sex_pdx::ring::PdxReply;
use libsys::pdx::{pdx_listen_raw, pdx_reply};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // sex-ld: Phase 21 Dynamic Linker
    // Replacement for standard ld.so in SASOS
    
    loop {
        // Wait for dynamic linking requests
        let req = pdx_listen_raw(0);
        
        // Safety: In this prototype, we assume the caller sent a LdProtocol message
        // In production, we'd validate the MessageType
        let msg = unsafe { *(req.arg0 as *const sex_pdx::LdProtocol) };
        
        let mut reply = PdxReply { status: 0, size: 0 };
        handle_ld_message(&msg, &mut reply);
        
        pdx_reply(req.caller_pd, &reply as *const _ as u64);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { libsys::sched::park_on_ring(); }
}
