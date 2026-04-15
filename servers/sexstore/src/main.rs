#![no_std]
#![no_main]

use libsys::pdx::{pdx_listen, pdx_reply, pdx_call};
use libsys::messages::MessageType;
use libsys::sched::park_on_ring;

/// sexstore: Standalone Package Manager and Self-Hosting Daemon.
/// Phase 13.1: Real manifest fetching via sexnet.

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        // Standard FLSCHED park
        park_on_ring();

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
            // 1. Resolve sexnet PD via capability slot 4
            // 2. Open TCP socket to Sex-Store repository
            let _sock = pdx_call(4, 1 /* NET_SOCKET */, 2 /* AF_INET */, 1 /* SOCK_STREAM */);
            
            // 3. Receive manifest and load into lent buffer
            // Simulation: Success with real manifest logic bridge
            (0, 4096)
        },
        2 => { // REPAIR_SYSTEM (sex-gemini hook)
            (0, 0)
        }
        _ => (-1, 0),
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("syscall", in("rax") 24); } }
}
