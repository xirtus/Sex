use crate::ipc::messages::MessageType;
use crate::ipc::{safe_pdx_call, DOMAIN_REGISTRY};
use crate::capability::{CapabilityData, MemLendCapData, DmaCapData};
use crate::core_local::CoreLocal;

/// kernel/src/syscalls/storage.rs
/// Phase 5: Bridge for Storage/DMA operations via PDX.
/// Maps kernel-level storage requests to the standalone sexdrives server.

pub fn sys_storage_read(device_cap: u32, offset: u64, size: u64, buffer_vaddr: u64) -> i64 {
    route_storage_call(1 /* FS_READ */, device_cap, offset, size, buffer_vaddr)
}

pub fn sys_storage_write(device_cap: u32, offset: u64, size: u64, buffer_vaddr: u64) -> i64 {
    route_storage_call(2 /* FS_WRITE */, device_cap, offset, size, buffer_vaddr)
}

fn route_storage_call(cmd: u32, device_cap: u32, offset: u64, size: u64, buffer_vaddr: u64) -> i64 {
    let current_pd = CoreLocal::get().current_pd_ref();
    
    // 1. Identify sexdrives PD (Hardcoded PD 200 for prototype bootstrap)
    let sexdrives_pd = match DOMAIN_REGISTRY.get(200) {
        Some(pd) => pd,
        None => return -1,
    };

    // 2. Create lent-memory capability for zero-copy DMA buffer
    let buffer_cap_id = sexdrives_pd.grant(CapabilityData::MemLend(MemLendCapData {
        base: buffer_vaddr,
        length: size,
        pku_key: current_pd.pku_key,
        permissions: 3, // R/W
    }));

    // 3. Construct DmaCall message
    let msg = MessageType::DmaCall {
        command: cmd,
        offset,
        size,
        buffer_cap: buffer_cap_id,
        device_cap,
    };

    // 4. Dispatch via pure PDX
    match safe_pdx_call(sexdrives_pd.as_ref(), 0, &msg as *const _ as u64) {
        Ok(res_ptr) => {
            let reply = unsafe { *(res_ptr as *const MessageType) };
            if let MessageType::DmaReply { status, .. } = reply {
                status
            } else {
                -1
            }
        },
        Err(_) => -1,
    }
}
