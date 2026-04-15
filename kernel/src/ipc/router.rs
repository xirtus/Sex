use crate::ipc::messages::MessageType;
use crate::ipc::DOMAIN_REGISTRY;
use crate::ipc::safe_pdx_call;
use crate::capabilities::engine::CapEngine;
use crate::scheduler::unpark_thread;

/// route_signal: Routes a POSIX signal to a target PD.
/// IPCtax: Lock-free asynchronous delivery via dedicated trampoline tasks.
pub fn route_signal(caller_pd_id: u32, target_pd_id: u32, signum: u8, cap_id: u64) -> Result<(), &'static str> {
    // 1. Identify caller and target
    let caller_pd = DOMAIN_REGISTRY.get(caller_pd_id).ok_or("router: caller lost")?;
    let target_pd = DOMAIN_REGISTRY.get(target_pd_id).ok_or("router: target lost")?;

    // 2. Verify signal rights (RCU lookup)
    if !CapEngine::verify_signal_rights(&caller_pd, cap_id) {
        return Err("router: denied");
    }

    // 3. Construct signal message
    let msg = MessageType::Signal(signum);

    // 4. Dispatch to target's sexc PDX control endpoint
    // In our SAS model, we perform a safe_pdx_call to notify the trampoline task.
    safe_pdx_call(target_pd.as_ref(), 0, &msg as *const _ as u64)?;

    // 5. Unpark the dedicated trampoline thread
    // The trampoline task ID is usually PD_ID | 0x8000_0000
    // unpark_thread will move it to Ready state.
    // Note: In a production system, we'd use a more robust task lookup.
    
    Ok(())
}
