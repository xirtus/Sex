#![no_std]
#![no_main]

mod vfs;
use vfs::handle_vfs_request;
use libsys::pdx::{pdx_listen, pdx_reply};
use libsys::messages::MessageType;
use libsys::sched::park_on_ring;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // sexvfs: Standalone Virtual File System Server
    // Phase 13.1: park_on_ring refactor.
    loop {
        // Blocks with FLSCHED::park() until VFS request arrives
        park_on_ring();

        let req = pdx_listen(0);
        let msg_ptr = req.arg0 as *const MessageType;
        let msg = unsafe { *msg_ptr };

        match msg {
            MessageType::VfsCall { command, offset, size, buffer_cap } => {
                let (status, res_size) = handle_vfs_request(command, offset, size, buffer_cap);
                let reply = MessageType::VfsReply { status, size: res_size };
                pdx_reply(req.caller_pd, &reply as *const _ as u64);
            },
            _ => {
                pdx_reply(req.caller_pd, u64::MAX);
            }
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { park_on_ring(); }
}
