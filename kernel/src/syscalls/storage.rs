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

fn route_storage_call(cmd: u32, device_cap_id: u32, offset: u64, size: u64, buffer_vaddr: u64) -> i64 {
    let current_pd = CoreLocal::get().current_pd_ref();
    
    // 1. Identify target via Pci device capability
    let device_cap = match current_pd.cap_table.find(device_cap_id) {
        Some(cap) => cap,
        None => return -1,
    };
    
    // In our model, Pci capabilities are usually held by the driver, 
    // but the application needs an IPC capability to the driver.
    // Let's assume device_cap_id is actually an IPC capability to the driver PD.
    let target_pd_id = match device_cap.data {
        CapabilityData::IPC(data) => data.target_pd_id,
        _ => return -1,
    };

    let sexdrives_pd = match DOMAIN_REGISTRY.get(target_pd_id) {
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
        device_cap: device_cap_id, // Pass the original capability if the driver needs it
    };

    // 4. Dispatch via pure PDX
    match safe_pdx_call(device_cap_id, &msg as *const _ as u64) {
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
