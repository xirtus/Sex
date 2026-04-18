#![no_std]
#![no_main]

extern crate alloc;
extern crate spin;
extern crate sex_rt;

mod pdx;

use crate::pdx::handle_ld_message;
use sex_pdx::ring::PdxReply;
use libsys::pdx::{pdx_listen, pdx_reply};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // sex-ld: Phase 21 Dynamic Linker
    // Replacement for standard ld.so in SASOS
    
    loop {
        // Wait for dynamic linking requests
        let req = pdx_listen(0);
        
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
