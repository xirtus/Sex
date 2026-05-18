# SEXNET_CAPABILITY_ROUTE_PROOF_V1

## A. Current Sexnet Route Truth
- **Spawn Status:** `sexnet` is statically spawned by `kernel/src/init.rs` as `domain_id=13`.
- **Capability Model:** Clients must use a capability slot ID (e.g., `SLOT_NET`) as the first argument to `pdx_call`, not the target's raw `domain_id`.
- **Missing ABI:** `SLOT_NET` (or `SLOT_NETWORK`) does not exist in `crates/sex-pdx/src/lib.rs`. `SLOT_LINEN` occupies slot 13.
- **Missing Grant:** `kernel/src/init.rs` does not grant any capability pointing to `sexnet` to `silk-shell` or any other application.
- **Dynamic Browser:** `kaleidoscope` is spawned dynamically by `silk-shell`. It cannot receive static boot grants from `init.rs` directly; it would require `silk-shell` to delegate the capability, which requires `silk-shell` to possess it first, and potentially collar approval.

## B. Smallest Safe Fix/Proof
**STOP FIRST.**
No safe, code-only patch exists that satisfies the constraints. Granting the slot requires modifying the `sex-pdx` crate ABI to define `SLOT_NET = 18` (or similar). Furthermore, the actual slot delegation model to dynamic applications like `kaleidoscope` requires architectural planning to ensure the Collar policy manager is respected.

## C. Files Changed
None (preflight documentation only).

## D. Proof Command
None executed.

## E. Result
**STOP**

## F. Next Mission
`SEXNET_SLOT_GRANT_PLAN_V1`
