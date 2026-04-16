use crate::ipc::messages::MessageType;
use crate::ipc::{safe_pdx_call, DOMAIN_REGISTRY};
use crate::capability::{CapabilityData, MemLendCapData};
use crate::core_local::CoreLocal;

/// kernel/src/syscalls/fs.rs
/// Phase 3: Bridge for VFS operations via PDX.
/// Maps kernel-level FS requests to the standalone sexvfs server.

pub fn sys_read(vfs_cap_id: u32, buffer: u64, size: u64) -> i64 {
    route_vfs_call(1, vfs_cap_id, size, buffer)
}

pub fn sys_write(vfs_cap_id: u32, buffer: u64, size: u64) -> i64 {
    route_vfs_call(2, vfs_cap_id, size, buffer)
}

fn route_vfs_call(cmd: u32, vfs_cap_id: u32, size: u64, buffer_vaddr: u64) -> i64 {
    let current_pd = CoreLocal::get().current_pd_ref();
    
    // 1. Resolve target via VFS capability (RCU-lookup)
    let vfs_cap = match unsafe { &*current_pd.cap_table }.find(vfs_cap_id) {
        Some(cap) => cap,
        None => return -1,
    };
    
    let target_pd_id = match vfs_cap.data {
        CapabilityData::Node(data) => data.sexdrive_pd_id,
        CapabilityData::IPC(data) => data.target_pd_id,
        _ => return -1,
    };
    
    let target_pd = match DOMAIN_REGISTRY.get(target_pd_id) {
        Some(pd) => pd,
        None => return -1,
    };

    // 2. Create lent-memory capability for zero-copy buffer
    let buffer_cap_id = target_pd.grant(CapabilityData::MemLend(MemLendCapData {
        base: buffer_vaddr,
        length: size,
        pku_key: current_pd.pku_key,
        permissions: 3, // R/W
        source_pd_id: current_pd.id,
    }));

    // 3. Dispatch via pure PDX
    let msg = MessageType::VfsCall { command: cmd, offset: 0, size, buffer_cap: buffer_cap_id };
    match safe_pdx_call(vfs_cap_id, &msg as *const _ as u64) {
        Ok(res_ptr) => {
            let reply = unsafe { *(res_ptr as *const MessageType) };
            if let MessageType::VfsReply { status, .. } = reply { status } else { -1 }
        },
        Err(_) => -1,
    }
}
