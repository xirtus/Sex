use crate::ipc::messages::MessageType;
use crate::ipc::DOMAIN_REGISTRY;
use crate::ipc::safe_pdx_call;
use crate::capabilities::engine::CapEngine;

/// route_signal: Routes a POSIX signal to a target PD.
/// IPCtax: Lock-free routing via safe_pdx_call.
pub fn route_signal(caller_pd_id: u32, target_pd_id: u32, signum: u8, cap_id: u64) -> Result<(), &'static str> {
    // 1. Identify caller and target
    let registry = &DOMAIN_REGISTRY;
    let caller_pd = registry.get(caller_pd_id).ok_or("router: caller lost")?;
    let target_pd = registry.get(target_pd_id).ok_or("router: target not found")?;

    // 2. Verify signal rights (RCU lookup)
    if !CapEngine::verify_signal_rights(&caller_pd, cap_id) {
        return Err("router: signal capability denied");
    }

    // 3. Construct signal message
    let msg = MessageType::Signal(signum);

    // 4. Dispatch to target's sexc PDX endpoint (Standalone ELF)
    // In our prototype, sexc handles its own PD's signals.
    safe_pdx_call(target_pd.as_ref(), 0 /* sexc control port */, &msg as *const _ as u64)?;

    Ok(())
}
