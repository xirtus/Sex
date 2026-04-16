use crate::ipc::messages::MessageType;
use crate::ipc::{safe_pdx_call, DOMAIN_REGISTRY};
use crate::capability::{CapabilityData, MemLendCapData};
use crate::core_local::CoreLocal;

/// kernel/src/syscalls/store.rs
/// Phase 13: Self-Hosting and Package Management via PDX to sexstore.

pub fn sys_store_fetch(store_cap_id: u32, name_ptr: u64, buffer_vaddr: u64, size: u64) -> i64 {
    let current_pd = CoreLocal::get().current_pd_ref();
    
    // 1. Identify target via Capability
    let store_cap = match unsafe { &*current_pd.cap_table }.find(store_cap_id) {
        Some(cap) => cap,
        None => return -1,
    };
    
    let target_pd_id = match store_cap.data {
        CapabilityData::IPC(data) => data.target_pd_id,
        _ => return -1,
    };

    let sexstore_pd = match DOMAIN_REGISTRY.get(target_pd_id) {
        Some(pd) => pd,
        None => return -1,
    };

    // 2. Create lent-memory capability for downloaded package
    let buffer_cap_id = sexstore_pd.grant(CapabilityData::MemLend(MemLendCapData {
        base: buffer_vaddr, length: size, pku_key: current_pd.pku_key, permissions: 3,
        source_pd_id: current_pd.id,
    }));

    // 3. Construct StoreCall message
    let msg = MessageType::StoreCall { command: 1, package_name_ptr: name_ptr, buffer_cap: buffer_cap_id };
    
    // 4. Dispatch via pure PDX
    match safe_pdx_call(store_cap_id, &msg as *const _ as u64) {
        Ok(res_ptr) => {
            let reply = unsafe { *(res_ptr as *const MessageType) };
            if let MessageType::StoreReply { status, size: fetched_size } = reply {
                if status == 0 { fetched_size as i64 } else { -1 }
            } else { -1 }
        },
        Err(_) => -1,
    }
}

