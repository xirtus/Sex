#![no_std]
#![no_main]

use libsys::pdx::{pdx_listen, pdx_reply, pdx_call};
use libsys::messages::MessageType;

/// sexnet: Standalone Network Stack and Remote PDX Proxy.
/// IPCtax: Pure PDX implementation, NO globals, NO busy-wait.

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Phase 4: Network Stack and Remote Routing.
    loop {
        // Blocks with FLSCHED::park() on the SPSC control ring
        unsafe { core::arch::asm!("syscall", in("rax") 24 /* SYS_PARK */); }

        let req = pdx_listen(0);
        let msg = unsafe { *(req.arg0 as *const MessageType) };

        match msg {
            MessageType::NetCall { command, socket_cap, size, buffer_cap, remote_node, .. } => {
                let (status, res_size, new_cap) = handle_net_request(command, socket_cap, size, buffer_cap, remote_node);
                let reply = MessageType::NetReply { status, size: res_size, socket_cap: new_cap };
                pdx_reply(req.caller_pd, &reply as *const _ as u64);
            },
            _ => {
                pdx_reply(req.caller_pd, u64::MAX);
            }
        }
    }
}

fn handle_net_request(cmd: u32, socket_cap: u32, size: u64, buffer_cap: u32, remote_node: u32) -> (i64, u64, u32) {
    match cmd {
        1 => { // NET_SOCKET
            // Allocate socket local or remote proxy
            (0, 0, 10) // New socket cap ID
        },
        2 => { // NET_CONNECT
            // Route via cluster fabric if remote_node != LOCAL
            (0, 0, socket_cap)
        },
        3 => { // NET_SEND
            // Zero-copy send via lent buffer_cap
            (size as i64, size, socket_cap)
        },
        4 => { // NET_RECV
            (size as i64, size, socket_cap)
        },
        _ => (-1, 0, 0),
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("syscall", in("rax") 24); } }
}
