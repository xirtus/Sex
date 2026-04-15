use alloc::sync::Arc;
use core::sync::atomic::{AtomicUsize, Ordering};

use sex_kernel::capability::ProtectionDomain;
use sex_kernel::ipc::DOMAIN_REGISTRY;
use sex_kernel::servers::sexc::{drain_pending_signals_for_pd, sexc, KernelSigAction};

extern crate alloc;

static HANDLER_FIRED: AtomicUsize = AtomicUsize::new(0);

extern "C" fn sigint_handler(sig: i32) {
    HANDLER_FIRED.fetch_add(sig as usize, Ordering::SeqCst);
}

#[test]
fn phase11_sigint_routes_into_user_handler() {
    HANDLER_FIRED.store(0, Ordering::SeqCst);

    let pd = Arc::new(ProtectionDomain::new(0x1100, 3));
    DOMAIN_REGISTRY.write().insert(pd.id, pd.clone());
    sex_kernel::servers::sexc::init_signal_trampoline(pd.id);

    let bridge = sexc::new(pd.id);
    let action = KernelSigAction {
        handler: sigint_handler as usize as u64,
        flags: 0,
        mask: 0,
        restorer: 0,
    };

    bridge
        .sigaction(2, &action as *const KernelSigAction as u64)
        .expect("sigaction should register");
    bridge.kill(pd.id, 2).expect("kill should route signal");

    assert_eq!(drain_pending_signals_for_pd(pd.id), 1);
    assert_eq!(HANDLER_FIRED.load(Ordering::SeqCst), 2);
}
