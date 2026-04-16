use crate::ipc::messages::MessageType;
use crate::ipc::DOMAIN_REGISTRY;
use crate::capabilities::engine::CapEngine;

/// route_signal: Routes a POSIX signal to a target PD.
/// IPCtax: Lock-free asynchronous delivery via message rings.
pub fn route_signal(caller_pd_id: u32, target_pd_id: u32, signum: u8, cap_id: u64) -> Result<(), &'static str> {
    // 1. Identify caller and target
    let caller_pd = DOMAIN_REGISTRY.get(caller_pd_id).ok_or("router: caller lost")?;
    let target_pd = DOMAIN_REGISTRY.get(target_pd_id).ok_or("router: target lost")?;

    // 2. Verify signal rights (RCU lookup)
    if !CapEngine::verify_signal_rights(caller_pd, cap_id) {
        return Err("router: denied");
    }

    // 3. Construct signal message
    let msg = MessageType::Signal(signum);

    // 4. Push to target's message ring
    unsafe {
        (*target_pd.message_ring).enqueue(msg).map_err(|_| "router: ring full")?;
    }

    // 5. Unpark the dedicated trampoline thread
    let trampoline_task = target_pd.trampoline_task.load(core::sync::atomic::Ordering::Acquire);
    if !trampoline_task.is_null() {
        crate::scheduler::unpark_thread(trampoline_task);
    }
    
    Ok(())
}
