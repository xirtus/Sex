#![no_std]
#![no_main]

use sex_pdx::{pdx_listen, pdx_reply, MessageType};
use libsys::sched::park_on_ring;

mod vfs;
mod messages;
mod pdx;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // sexfiles: Advanced PDX Zero-Copy VFS Server
    // Phase 18: Migration from legacy sexvfs to pure Rust sexfiles.
    loop {
        // Block until VFS request arrives
        park_on_ring();

        let req = pdx_listen(0);
        let msg_ptr = req.arg0 as *const MessageType;
        let msg = unsafe { &*msg_ptr };

        let reply = vfs::handle_vfs_request(msg);
        pdx_reply(req.caller_pd, &reply as *const _ as u64);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { park_on_ring(); }
}
