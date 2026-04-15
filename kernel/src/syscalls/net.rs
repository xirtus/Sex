use crate::ipc::messages::MessageType;
use crate::ipc::{safe_pdx_call, DOMAIN_REGISTRY};
use crate::capability::{CapabilityData, MemLendCapData};
use crate::core_local::CoreLocal;

/// kernel/src/syscalls/net.rs
/// Phase 4: Bridge for Network operations via PDX.
/// Maps kernel-level NET requests to the standalone sexnet server.

pub fn sys_socket(net_cap_id: u32, domain: i32, net_type: i32, protocol: i32) -> i64 {
    route_net_call(1, net_cap_id, 0, 0, 0, 0) // Treat socket_cap as net_cap_id for initial socket creation
}

pub fn sys_send(socket_cap_id: u32, buffer: u64, size: u64) -> i64 {
    route_net_call(3, socket_cap_id, socket_cap_id, size, buffer, 0)
}

fn route_net_call(cmd: u32, target_cap_id: u32, socket_cap_id: u32, size: u64, buffer_vaddr: u64, remote_node: u32) -> i64 {
    let current_pd = CoreLocal::get().current_pd_ref();
    
    // 1. Identify target PD via capability
    let target_cap = match current_pd.cap_table.find(target_cap_id) {
        Some(cap) => cap,
        None => return -1,
    };
    
    let target_pd_id = match target_cap.data {
        CapabilityData::IPC(data) => data.target_pd_id,
        _ => return -1,
    };

    let sexnet_pd = match DOMAIN_REGISTRY.get(target_pd_id) {
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
            source_pd_id: current_pd.id,
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
    match safe_pdx_call(target_cap_id, &msg as *const _ as u64) {
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
