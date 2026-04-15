use crate::ipc::route_signal;

#[test]
fn test_signal_delivery() {
    // Phase 6: Verify lock-free signal routing and trampoline dispatch
    let target_pd_id = 3000;
    let signo = 10; // SIGUSR1
    let sender_cap_id = 1;

    let res = route_signal(target_pd_id as u64, signo, sender_cap_id);
    assert_eq!(res, 0, "Signal routing failed");
}
