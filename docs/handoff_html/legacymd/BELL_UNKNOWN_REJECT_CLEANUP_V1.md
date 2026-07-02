# BELL_UNKNOWN_REJECT_CLEANUP_V1

**Status:** Complete — temporary kernel test enqueue removed. sexbell spawn + self-cap preserved.
**Build:** `[SEXOS ENTRYPOINT] success` — ISO produced.
**Date:** 2026-05-05
**Depends on:** `BELL_UNKNOWN_REJECT_PROOF_V1.md` (proof that was cleaned up)

---

## Summary

Removed the temporary one-shot kernel test message enqueue (`[kernel.sexbell.test]` + `0xFFFF` IpcCall) from `kernel/src/init.rs`. This was proof scaffolding from `BELL_UNKNOWN_REJECT_PROOF_V1` and must not remain as retained boot behavior.

## Removed Code

The following was removed from `kernel/src/init.rs` (lines ~179-197):

```rust
// Bell unknown-reject proof: send one controlled test PDX message with unknown opcode.
// This enqueues an IpcCall on sexbell's message ring before sexbell starts its listen loop.
// sexbell will dequeue it, fail to match any OP_BELL_* arm, and emit [bell.unknown.reject].
// No external cap grant needed — kernel enqueues directly via message_ring.
if sexbell_id != 0 {
    use crate::ipc::DOMAIN_REGISTRY;
    use crate::ipc::messages::MessageType;
    if let Some(pd) = DOMAIN_REGISTRY.get(sexbell_id) {
        let test_msg = MessageType::IpcCall {
            func_id: 0xFFFF,
            arg0: 0, arg0: 0, arg1: 0,
            caller_pd: 0,
        };
        unsafe { let _ = (*pd.message_ring).enqueue(test_msg); }
        serial_println!("[kernel.sexbell.test] op=0xFFFF slot={}", sex_pdx::SLOT_BELL);
    }
}
```

## Preserved

| Feature | Status | Location |
|---------|--------|----------|
| sexbell spawn | ✅ | `init.rs:39` module_paths, `init.rs:81-82` domain-10 capture |
| SLOT_BELL self-cap | ✅ | `init.rs:169-175` |
| sexbell stub behavior | ✅ | `servers/sexbell/src/main.rs` — boot + unknown reject |
| All existing server spawns | ✅ | Unchanged |

## Files Changed

| File | Change | Type |
|------|--------|------|
| `kernel/src/init.rs` | Removed test enqueue block (~12 lines) | Code |
| `docs/handoff/BELL_UNKNOWN_REJECT_CLEANUP_V1.md` | New handoff doc | Doc |

## Validation

| Check | Result |
|-------|--------|
| sexbell spawn | ✅ `let mut sexbell_id = 0;`, `module_paths` entry, domain-10 capture |
| SLOT_BELL self-cap | ✅ `grant_capability(SLOT_BELL, ...)` |
| No test enqueue | ✅ No `[kernel.sexbell.test]` or `0xFFFF` in init.rs |
| Build | ✅ `[SEXOS ENTRYPOINT] success` |

## Next Phase Recommendation

**BELL_NOTIFY_PLAN_V1** — Docs-only plan for the first Bell protocol crossing. Must decide:
- Which PD sends the first `OP_BELL_NOTIFY` test message
- What cap path is required
- Whether sexbell should parse the message or continue rejecting
- Whether sexbell needs to emit a new proof marker `[bell.notify.recv]`

No implementation until plan is approved.

## References

- `BELL_UNKNOWN_REJECT_PROOF_V1.md` — original proof (now cleaned up)
- `BELL_BOOT_SPAWN_V1.md` — kernel spawn implementation
- `kernel/src/init.rs` — cleaned, no test enqueue

---

*End of BELL_UNKNOWN_REJECT_CLEANUP_V1.md*
