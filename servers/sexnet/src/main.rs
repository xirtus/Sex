#![no_std]
#![no_main]

use libsys::pdx::{pdx_listen, pdx_reply};
use sex_pdx::MessageType;
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
            MessageType::Node(proto) => match proto {
                sex_pdx::NodeProtocol::ClusterObjectFetch { node_id: _, hash: _ } => {
                    // Send XIPC packet to remote node_id for object retrieval
                    // Mock: resolve from remote mock node
                    pdx_reply(req.caller_pd, 0x9000 /* Mock Remote PFN */);
                },
                sex_pdx::NodeProtocol::CapabilityResolve { name: _ } => {
                    // XIPC Broadcast to all known cluster nodes
                    // Mock: Node 2 has the service 'sexfiles'
                    pdx_reply(req.caller_pd, 2 << 32 | 1 /* NodeID 2, LocalID 1 */);
                },
                _ => pdx_reply(req.caller_pd, u64::MAX),
            },
            _ => pdx_reply(req.caller_pd, u64::MAX),
        }
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { park_on_ring(); }
}
