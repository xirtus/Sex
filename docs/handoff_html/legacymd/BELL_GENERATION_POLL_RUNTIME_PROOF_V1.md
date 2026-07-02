# BELL_GENERATION_POLL_RUNTIME_PROOF_V1

## Status: PASS

## Proof Chain

```
[bell.boot]                              ← Bell starts, init queue
[bell.subscribe.reply] gen=1             ← Bell replies to SUBSCRIBE
[silkbar.bell.gen.reply] gen=1 changed=1  ← SilkBar sees gen 0→1, calls LIST
[bell.list.reply] total=0 lanes=[...]     ← LIST returns empty queue
[silkbar.bell.poll.reply] total=0 ...    ← SilkBar forwards to sexdisplay
[sexdisplay.bell.render] total=0 ...     ← sexdisplay renders bell presence
[bell.subscribe.reply] gen=1             ← second SUBSCRIBE (budget 4)
[silkbar.bell.gen.reply] gen=1 changed=0 ← steady-state: skip LIST
[bell.subscribe.reply] gen=1             ← third SUBSCRIBE
[bell.subscribe.reply] gen=1             ← fourth SUBSCRIBE (budget exhausted)
→ silence (no further markers)
```

## Verification

| Check | Result |
|-------|--------|
| `[bell.subscribe.reply]` present | ✅ |
| `[silkbar.bell.gen.reply] changed=1` present | ✅ |
| `[bell.list.reply]` follows changed=1 | ✅ |
| `[silkbar.bell.poll.reply]` follows list | ✅ |
| `[sexdisplay.bell.render]` present | ✅ |
| `[silkbar.bell.gen.reply] changed=0` appears once | ✅ |
| `[silkbar.bell.gen.fallback]` absent | ✅ |
| `[bell.subscribe.deny]` absent | ✅ |
| `[bell.list.reject]` absent | ✅ |
| Steady-state silence (no changed=0 spam) | ✅ |

## Root Cause Fix (prior commit)

`OP_BELL_LIST` was missing from the `use sex_pdx::{...}` import in
`servers/sexbell/src/main.rs`.  Rust treats un-imported identifiers in
match arms as catch-all variable bindings, making the `OP_BELL_LIST`
arm match every value and shadow all subsequent arms
(`OP_BELL_CLOSE`, `OP_BELL_SUBSCRIBE`).  The SUBSCRIBE handler
compiled but was dead code — LTO eliminated it entirely (confirmed by
`strings` + `objdump` showing no `[bell.subscribe.reply]` or
`[bell.subscribe.deny]` strings in the binary).

Fix: add `OP_BELL_LIST` to the import.

## Marker Noise Cleanup

### sexbell (BELL_SUBSCRIBE_REPLY_BUDGET)

Reduced from 8 → 4.  The subscribe-reply marker confirms the SUBSCRIBE
handshake works; SilkBar's `[silkbar.bell.gen.reply]` carries the same
information.  4 emissions are sufficient for boot-time proof.

### silkbar (gen.reply split)

Split single `BELL_GEN_REPLY_BUDGET` (8) into two budgets:

- `BELL_GEN_REPLY_CHANGED_BUDGET`: 8 (preserve changed=1 signal)
- `BELL_GEN_REPLY_STEADY_BUDGET`: 1 (single "steady state confirmed"
  print, then silence)

This eliminates the `changed=0` spam (was emitting every 2s for 16s
with old budget of 8).

## Files Changed

| File | Change |
|------|--------|
| `servers/sexbell/src/main.rs` | Budget 8→4 for subscribe.reply |
| `servers/silkbar/src/main.rs` | Split gen.reply budget; steady=1 |
| `docs/handoff/BELL_GENERATION_POLL_RUNTIME_PROOF_V1.md` | Created |

## Build

`./scripts/entrypoint_build.sh` passes.

## Next Recommended Phase

1. **BELL_PHASE_E2_POLICY**: Implement `OP_BELL_SET_POLICY` for
   dynamic read-cap allowlist and per-caller privacy levels, as
   designed in `BELL_PHASE_E_SUBSCRIBE_POLICY_DESIGN_V1.md`.
2. **BELL_SILKBAR_CRASH_RECOVERY**: Handle Bell restart without
   stale generation state in SilkBar (reset `bell_gen_cached` on
   subscribe failure).
