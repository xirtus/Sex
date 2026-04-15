use crate::xipc::messages::MessageType;
use crate::ipc::DOMAIN_REGISTRY;

/// forward_page_fault: Routes #PF to the sext server.
/// IPCtax mandate: Async forwarding via ring buffer to avoid kernel stack bloat.
pub fn forward_page_fault(fault_addr: u64, error_code: u32, pd_id: u64) -> Result<(), &'static str> {
    // 1. Identify the sext PD (usually PID 2 in our prototype)
    let registry = DOMAIN_REGISTRY.read();
    let sext_pd = registry.get(&2).ok_or("sext: Pager not found")?;

    // 2. Construct MessageType::PageFault
    let msg = MessageType::PageFault {
        fault_addr,
        error_code,
        pd_id,
    };

    // 3. Enqueue to sext's control ring
    if let Some(ref sexc_state) = sext_pd.sexc_state.lock().as_ref() {
        if sexc_state.control_ring.enqueue(msg).is_err() {
            return Err("sext: Control ring full");
        }

        // 4. Wake sext
        crate::scheduler::unpark_thread(sexc_state.trampoline_tid);
    } else {
        return Err("sext: Not initialized");
    }

    Ok(())
}
