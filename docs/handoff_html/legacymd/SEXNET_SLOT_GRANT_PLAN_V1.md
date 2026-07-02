# SEXNET_SLOT_GRANT_PLAN_V1

## A. Recommended Capability Route
The correct architectural route under SexOS is a **per-app delegated capability from `silk-shell`**.
1. **Boot Grant:** The kernel statically grants `SLOT_NET` pointing to `sexnet_id` (domain 13) to the `silk-shell` orchestration PD during boot (`kernel/src/init.rs`).
2. **Delegation:** When `silk-shell` spawns dynamic applications (like `kaleidoscope`), it delegates `SLOT_NET` to the child PD.
3. **Policy:** Long-term, this delegation will be gated by the `Collar` policy server.

## B. Minimal ABI/Cap Changes Required
1. **ABI:** Add `pub const SLOT_NET: u64 = 18;` to `crates/sex-pdx/src/lib.rs`.
2. **Kernel Init:** Add `pd.grant_capability(sex_pdx::SLOT_NET, CapabilityData::Domain(sexnet_id));` to `silk-shell`'s boot-time grant block in `kernel/src/init.rs`.
3. **Shell IPC:** Update `silk-shell` to perform a test `pdx_call(SLOT_NET, SEXNET_GET_STATUS, 0, 0, 0)`.

## C. Files Likely Touched
- `crates/sex-pdx/src/lib.rs` (ABI definition)
- `kernel/src/init.rs` (Static boot capability grant)
- `servers/silk-shell/src/main.rs` (Shell-level diagnostic proof and future delegation)

## D. STOP FIRST Boundaries
- **No pointer-copy:** MPK/PKU makes blind cross-PD pointers fatal. Do not use or fix `SEXNET_SCAN_WIFI`'s unsafe pointer logic.
- **No browser IPC yet:** The browser cannot receive `SLOT_NET` until `silk-shell` proves it can hold and use the capability itself.
- **No kernel shared-memory redesign:** We must not alter the kernel IPC mechanics to support heavy text transport.

## E. Phase Sequence
1. **SLOT_NET_CONSTANT_AND_GRANT_PROOF_V1:** Define `SLOT_NET=18`, grant to `silk-shell` at boot, and prove `silk-shell` can make a scalar `SEXNET_GET_STATUS` call safely.
2. **BROWSER_NETWORK_DELEGATION_PROOF_V1:** `silk-shell` safely delegates `SLOT_NET` to `kaleidoscope`. The browser proves it can make the same scalar `GET_STATUS` call.
3. **SEXNET_SAFE_SMALL_REPLY_PROOF_V1:** Implement a register-packed text transfer model (e.g., passing 8 bytes of string data per scalar `pdx_call` return register) to bypass MPK pointer violations without allocating shared memory buffers.
4. **BROWSER_LIVE_RENDER_PROOF_V1:** The browser queries text chunks via the safe scalar ABI and renders them to the screen.

## F. Gemini Prompt for Phase 1 Only

```bash
cat > /tmp/gemini_slot_net_constant_and_grant_proof_v1.prompt <<'EOF'
MISSION: SLOT_NET_CONSTANT_AND_GRANT_PROOF_V1

Goal:
Add `SLOT_NET` ABI constant, grant it to `silk-shell` statically, and prove the route works with a safe scalar call.

Constraints:
NO pointer-copy.
NO Kaleidoscope changes yet.
NO sexnet functional changes.
Strict no_std, MPK/PKU isolation.

Task:
1. Add `pub const SLOT_NET: u64 = 18;` to `crates/sex-pdx/src/lib.rs`.
2. In `kernel/src/init.rs`, inside the `silk-shell` capability grant block, add:
   `pd.grant_capability(sex_pdx::SLOT_NET, CapabilityData::Domain(sexnet_id));`
3. In `servers/silk-shell/src/main.rs`, perform a test call: `pdx_call(SLOT_NET, 0x200 /* GET_STATUS */, 0, 0, 0)` and print the scalar status value.
4. Run the daily driver proof and verify the `[sexnet.route.slot.grant]` and `[sexnet.route.call.get_status]` markers appear.

Output:
- files changed
- PASS/SKIP/STOP
- log evidence
- Next mission: BROWSER_NETWORK_DELEGATION_PROOF_V1
EOF
```