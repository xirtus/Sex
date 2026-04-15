use crate::ipc::messages::MessageType;
use crate::ipc::{safe_pdx_call, DOMAIN_REGISTRY};
use crate::capability::{CapabilityData, MemLendCapData};
use crate::core_local::CoreLocal;

/// kernel/src/syscalls/translator.rs
/// Phase 12: execve translation path via PDX to standalone sexnode.

pub fn sys_translate_and_exec(translator_cap_id: u32, path_ptr: u64, code_vaddr: u64, size: u64) -> i64 {
    let current_pd = CoreLocal::get().current_pd_ref();
    
    // 1. Identify target via Capability
    let trans_cap = match current_pd.cap_table.find(translator_cap_id) {
        Some(cap) => cap,
        None => return -1,
    };
    
    let target_pd_id = match trans_cap.data {
        CapabilityData::IPC(data) => data.target_pd_id,
        _ => return -1,
    };

    let sexnode_pd = match DOMAIN_REGISTRY.get(target_pd_id) {
        Some(pd) => pd,
        None => return -1,
    };

    // 2. Create lent-memory capability for binary code
    let code_cap_id = sexnode_pd.grant(CapabilityData::MemLend(MemLendCapData {
        base: code_vaddr,
        length: size,
        pku_key: current_pd.pku_key,
        permissions: 1, // Read-only
    }));

    // 3. Construct TranslatorCall message
    let msg = MessageType::TranslatorCall {
        command: 1, // TRANSLATE_ELF
        path_ptr,
        code_cap: code_cap_id,
    };

    // 4. Dispatch via pure PDX
    match safe_pdx_call(translator_cap_id, &msg as *const _ as u64) {
        Ok(res_ptr) => {
            let reply = unsafe { *(res_ptr as *const MessageType) };
            if let MessageType::TranslatorReply { status, translated_entry } = reply {
                if status == 0 {
                    // 5. Spawn new PD with translated entry point
                    // In real system, this integrates with sys_spawn_pd
                    translated_entry as i64
                } else {
                    -1
                }
            } else {
                -1
            }
        },
        Err(_) => -1,
    }
}
