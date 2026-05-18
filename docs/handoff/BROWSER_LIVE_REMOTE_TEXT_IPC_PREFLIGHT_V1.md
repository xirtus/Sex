# BROWSER_LIVE_REMOTE_TEXT_IPC_PREFLIGHT_V1

## A. sexnet spawn status
**Passively spawned.** `kernel/src/init.rs` spawns `sexnet` as a passive status-only PD (`domain_id` 13). It has `slot_net_grant=0` and no real NIC access.

## B. sexnet PD/slot truth
**Mismatched / Not pinned safely.** `kernel/src/init.rs` spawns `sexnet` dynamically at index 12 (Domain ID 13). However, `crates/silknet/src/lib.rs` hardcodes `SEXNET_PD = 5`. Furthermore, older docs reference Slot 4 or Slot 2. The routing ID is unpinned and currently broken.

## C. pointer-copy safety evidence
**UNPROVEN.** `SEXNET_SCAN_WIFI` exists in code but has no runtime proof. Under the strict MPK/PKU isolation currently enforced in SexOS, blindly copying pointers across Protection Domains is unsafe. A secure cross-PD data transfer requires an explicitly mapped shared memory buffer or a robust ABI addition, neither of which exists for `sexnet`.

## D. pdx_call availability
**Accessible, but route is blocked.** `sex_pdx::pdx_call` (5 arguments) is available in the `sex-pdx` crate and can be imported by Kaleidoscope without dependency changes. However, calling it will fail because Kaleidoscope does not have a `SLOT_NET` capability grant to reach `sexnet`.

## E. GO or STOP
**STOP.**
The implementation would require kernel/ABI edits to create a safe shared memory IPC route for the payload string, and the `sexnet` PD ID is severely mismatched.

## F. Next Mission
`BROWSER_LIVE_REMOTE_TEXT_IPC_PLAN_V1`
