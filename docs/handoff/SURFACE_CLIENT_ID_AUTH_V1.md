# SURFACE_CLIENT_ID_AUTH_V1

Date: 2026-05-07
Status: LANDED

## Files Changed

- `servers/sexdisplay/src/main.rs`
- `servers/silk-shell/src/main.rs`

No kernel changes. No sex-pdx ABI changes.

## What Changed

### sexdisplay

1. Added `static DISPLAY_WM_PD: AtomicU32` — stores kernel-verified PD of registered WM.

2. Added `0xF5` (OP_REGISTER_WM) handler — first caller wins; idempotent for same caller.
   - Proof markers: `[sexdisplay.auth.wm.register]`, `[sexdisplay.auth.wm.deny]`

3. Fixed `0xED` (OP_SET_FOCUS) — was fully open; now owner OR registered WM only.
   - sid=0 (clear focus) passes through unconditionally (benign).
   - Deny marker: `[sexdisplay.auth.deny] op=0xed`

4. Fixed `0xEB` (OP_SURFACE_UPDATE/move) — was owner-only; added WM bypass.
   - Required so silk-shell can move/reposition client surfaces as WM.
   - Deny marker: `[sexdisplay.auth.deny] op=0xeb`

5. Fixed `0xFD` (OP_SURFACE_TAB_INFO) — was unguarded; now owner OR WM.
   - Required so shell can set tab chrome on any managed surface.
   - Deny marker: `[sexdisplay.auth.deny] op=0xfd`

6. `0xEE` (OP_SURFACE_DESTROY) unchanged — owner only. WM cannot destroy client surfaces (V1 policy).

### silk-shell

- Added `const OP_REGISTER_WM: u64 = 0xF5;` (local, not exported to sex-pdx).
- Calls `pdx_call(SLOT_DISPLAY, OP_REGISTER_WM, 0, 0, 0)` once at startup after
  `sys_set_state(SVC_STATE_LISTENING)`, before SilkBar advertisement.

## Authorization Matrix

| Opcode | Operation       | Owner | Registered WM | Other |
|--------|-----------------|-------|---------------|-------|
| 0xEC   | create/upsert   | ✅    | ✅ (own)       | ❌    |
| 0xEB   | move/update     | ✅    | ✅ (WM policy) | ❌    |
| 0xED   | focus/raise     | ✅    | ✅ (WM policy) | ❌    |
| 0xEE   | destroy         | ✅    | ❌ (V1 policy) | ❌    |
| 0xEF   | fill rect       | ✅    | ❌             | ❌    |
| 0xFA   | text clear      | ✅    | ❌             | ❌    |
| 0xFB   | text draw       | ✅    | ❌             | ❌    |
| 0xFD   | tab chrome      | ✅    | ✅ (WM policy) | ❌    |
| 0xF5   | register WM     | n/a   | first-wins     | ❌    |

## Identity Model

- `caller_pd` is kernel-injected and unforgeable (set from `current_pd.id` in ipc.rs before message delivery).
- Client cannot pass a fake `caller_pd` — it is not a user argument.
- `DISPLAY_WM_PD` is set once via compare_exchange(0, caller). Subsequent callers rejected.
- silk-shell registers immediately after init, before any surface management begins.

## Expected Proof Markers (boot serial)

```
[sexdisplay.auth.wm.register] caller=<N> ok=1
```

Where `<N>` is silk-shell's kernel-assigned PD id (not assumed to equal SLOT_SHELL=6).

## Invariant (Post-V1)

- Owner controls lifecycle and content of own surface.
- Registered WM controls focus and placement of any surface.
- sexdisplay only renders — no authority decisions outside this file.
- No client PD can focus, move, or set chrome on another client's surface unless it is the registered WM.
- No PD (including WM) can destroy a surface it does not own.

## Open: V1.1 Blocker

None. 0xFD was guarded in this patch (trivial, ≤15 lines). No deferred holes remain in surface auth.

## Risks

| # | Risk | Status |
|---|------|--------|
| R1 | SHELL_PD constant brittle | Resolved — no constant used; kernel identity only |
| R2 | 0xFD unguarded | Resolved — guarded in this patch |
| R3 | WM bypass on 0xEB | Resolved — bypass scoped to registered WM only |
| R4 | Shell destroy client surfaces | Blocked by V1 policy (0xEE unchanged) |
| R5 (HIGH, accepted) | Boottime race: any PD can call 0xF5 before silk-shell and claim WM identity. Accepted — boot order is deterministic and no malicious PD exists in current image. V2 mitigation: kernel-enforced privilege bit on OP_REGISTER_WM. | Open/accepted |
