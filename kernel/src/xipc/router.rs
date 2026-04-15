use crate::capability::ProtectionDomain;
use crate::capabilities::engine::CapEngine;
use crate::ipc::DOMAIN_REGISTRY;
use super::messages::MessageType;

/// route_signal: IPCtax-compliant signal routing.
/// Performs capability verification and zero-copy enqueue into target PD's control ring.
pub fn route_signal(target_pd_id: u64, signo: u32, sender_cap_id: u64) -> i32 {
    let registry = DOMAIN_REGISTRY.read();
    let target_pd = match registry.get(&(target_pd_id as u32)) {
        Some(pd) => pd,
        None => return -1, // ESRCH
    };

    // 1. Capability check via CapEngine
    if !CapEngine::verify_signal_rights(&target_pd, sender_cap_id) {
        return -2; // EPERM
    }

    // 2. Zero-copy enqueue into the target's control SPSC ring
    let msg = MessageType::Signal { signo, sender_capability_id: sender_cap_id };
    
    let sexc_state_lock = target_pd.sexc_state.lock();
    if let Some(ref sexc_state) = *sexc_state_lock {
        if sexc_state.control_ring.enqueue(msg).is_err() {
            return -3; // EAGAIN (Ring full)
        }

        // 3. FLSCHED wake of the trampoline thread
        crate::scheduler::unpark_thread(sexc_state.trampoline_tid);
    } else {
        return -4; // ENOSYS (Trampoline not initialized)
    }

    0
}
