use crate::ipc::messages::MessageType;
use crate::ipc::{safe_pdx_call, DOMAIN_REGISTRY};
use crate::capability::{CapabilityData, MemLendCapData};
use crate::core_local::CoreLocal;

/// kernel/src/syscalls/store.rs
/// Phase 13: Self-Hosting and Package Management via PDX to sexstore.

pub fn sys_store_fetch(name_ptr: u64, buffer_vaddr: u64, size: u64) -> i64 {
    let current_pd = CoreLocal::get().current_pd_ref();
    
    // 1. Identify sexstore PD (Hardcoded PD 700 for prototype)
    let sexstore_pd = match DOMAIN_REGISTRY.get(700) {
        Some(pd) => pd,
        None => return -1,
    };

    // 2. Create lent-memory capability for downloaded package
    let buffer_cap_id = sexstore_pd.grant(CapabilityData::MemLend(MemLendCapData {
        base: buffer_vaddr,
        length: size,
        pku_key: current_pd.pku_key,
        permissions: 3, // R/W
    }));

    // 3. Construct StoreCall message
    let msg = MessageType::StoreCall {
        command: 1, // FETCH_PACKAGE
        package_name_ptr: name_ptr,
        buffer_cap: buffer_cap_id,
    };

    // 4. Dispatch via pure PDX
    match safe_pdx_call(sexstore_pd.as_ref(), 0, &msg as *const _ as u64) {
        Ok(res_ptr) => {
            let reply = unsafe { *(res_ptr as *const MessageType) };
            if let MessageType::StoreReply { status, size: fetched_size } = reply {
                if status == 0 { fetched_size as i64 } else { -1 }
            } else {
                -1
            }
        },
        Err(_) => -1,
    }
}
