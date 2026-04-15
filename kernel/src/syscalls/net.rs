use crate::ipc::messages::MessageType;
use crate::ipc::{safe_pdx_call, DOMAIN_REGISTRY};
use crate::capability::{CapabilityData, MemLendCapData};
use crate::core_local::CoreLocal;

/// kernel/src/syscalls/net.rs
/// Phase 4: Bridge for Network operations via PDX.
/// Maps kernel-level NET requests to the standalone sexnet server.

pub fn sys_socket(domain: i32, net_type: i32, protocol: i32) -> i64 {
    route_net_call(1, 0, 0, 0, 0)
}

pub fn sys_send(socket_cap_id: u32, buffer: u64, size: u64) -> i64 {
    route_net_call(3, socket_cap_id, size, buffer, 0)
}

fn route_net_call(cmd: u32, socket_cap_id: u32, size: u64, buffer_vaddr: u64, remote_node: u32) -> i64 {
    let current_pd = CoreLocal::get().current_pd_ref();
    
    // 1. Identify sexnet PD (Hardcoded PD 400 for prototype bootstrap)
    let sexnet_pd = match DOMAIN_REGISTRY.get(400) {
        Some(pd) => pd,
        None => return -1,
    };

    // 2. Create lent-memory capability for zero-copy buffer if size > 0
    let buffer_cap_id = if size > 0 {
        sexnet_pd.grant(CapabilityData::MemLend(MemLendCapData {
            base: buffer_vaddr,
            length: size,
            pku_key: current_pd.pku_key,
            permissions: 1, // Read-only for send, R/W for recv
        }))
    } else {
        0
    };

    // 3. Construct NetCall message
    let msg = MessageType::NetCall {
        command: cmd,
        socket_cap: socket_cap_id,
        offset: 0,
        size,
        buffer_cap: buffer_cap_id,
        remote_node,
    };

    // 4. Dispatch via pure PDX
    match safe_pdx_call(sexnet_pd.as_ref(), 0, &msg as *const _ as u64) {
        Ok(res_ptr) => {
            let reply = unsafe { *(res_ptr as *const MessageType) };
            if let MessageType::NetReply { status, socket_cap, .. } = reply {
                if cmd == 1 { socket_cap as i64 } else { status }
            } else {
                -1
            }
        },
        Err(_) => -1,
    }
}
