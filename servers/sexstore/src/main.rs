#![no_std]
#![no_main]

use libsys::pdx::{pdx_listen, pdx_reply};
use libsys::messages::MessageType;

/// sexstore: Standalone Package Manager and Self-Hosting Daemon.
/// Phase 13: Full Sex-in-Sex environment bootstrapping.

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        // Blocks with FLSCHED::park() on the SPSC control ring
        unsafe { core::arch::asm!("syscall", in("rax") 24 /* SYS_PARK */); }

        let req = pdx_listen(0);
        let msg = unsafe { *(req.arg0 as *const MessageType) };

        match msg {
            MessageType::StoreCall { command, package_name_ptr, buffer_cap } => {
                let (status, size) = handle_store_request(command, package_name_ptr, buffer_cap);
                let reply = MessageType::StoreReply { status, size };
                pdx_reply(req.caller_pd, &reply as *const _ as u64);
            },
            _ => {
                pdx_reply(req.caller_pd, u64::MAX);
            }
        }
    }
}

fn handle_store_request(cmd: u32, _name_ptr: u64, _buf_cap: u32) -> (i64, u64) {
    match cmd {
        1 => { // FETCH_PACKAGE
            // 1. Resolve package name from lent memory (via PDX if necessary)
            // 2. Fetch package binary via sexnet / sexvfs
            // 3. Extract and load into lent buffer_cap
            // Simulated success: Return written size
            (0, 4096)
        },
        2 => { // REPAIR_SYSTEM (sex-gemini hook)
            // Initiate full self-rebuild or consistency check
            (0, 0)
        }
        _ => (-1, 0),
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("syscall", in("rax") 24); } }
}
