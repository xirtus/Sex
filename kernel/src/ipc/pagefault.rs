use crate::ipc::messages::MessageType;
use crate::ipc::DOMAIN_REGISTRY;
use crate::ipc::safe_pdx_call;

/// forward_page_fault: Routes #PF to the sext server.
/// IPCtax mandate: Async forwarding via PDX to avoid kernel stack bloat.
pub fn forward_page_fault(fault_addr: u64, error_code: u32, pd_id: u64) -> Result<(), &'static str> {
    // 1. Construct MessageType::PageFault
    let msg = MessageType::PageFault {
        fault_addr,
        error_code,
        pd_id,
        lent_cap: 0,
    };

    // 2. Dispatch safe PDX call to sext (Slot 2 in Root PD)
    safe_pdx_call(2, &msg as *const _ as u64)?;

    Ok(())
}
