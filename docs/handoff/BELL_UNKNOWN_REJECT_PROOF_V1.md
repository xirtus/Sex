# BELL_UNKNOWN_REJECT_PROOF_V1

**Status:** Complete — sexbell received and rejected one controlled unknown PDX message.
**Build:** `[SEXOS ENTRYPOINT] success` — ISO produced.
**Boot:** QEMU (30s timeout), serial log captured.
**Date:** 2026-05-05
**Depends on:** `BELL_SPAWN_PROOF_V1.md` (boot proof)

---

## Summary

Sent one controlled PDX message with unknown opcode `0xFFFF` to sexbell from the kernel init path. sexbell's listen loop dequeued the message, failed to match any `OP_BELL_*` arm, and emitted `[bell.unknown.reject]`. No faults, no panics, no protocol parsing, no side effects.

---

## Test Method

**Sender:** Kernel init path (`kernel/src/init.rs`)

**Why kernel init:** The kernel has direct access to PD message rings. No cap grants needed — the kernel enqueues a `MessageType::IpcCall` directly onto sexbell's message ring. This avoids editing silk-shell or any other userspace PD.

**Test message:**

| Field | Value | Rationale |
|-------|-------|-----------|
| `func_id` (opcode) | `0xFFFF` | Deliberately unknown — not in assigned Bell range `0xC0-0xC7` |
| `arg0`, `arg1`, `arg2` | `0` | Harmless zero constants |
| `caller_pd` | `0` | Kernel-originated |

**Timing:** The message is enqueued during init, before the scheduler starts sexbell. When sexbell's `_start` runs and enters its listen loop, the message is the first item dequeued.

---

## Proof Markers Found

| Marker | Line | Expected | Found |
|--------|------|----------|-------|
| `[kernel.sexbell.test]` | 716 | ✅ | ✅ op=0xFFFF slot=12 |
| `[bell.boot]` | 986 | ✅ | ✅ sexbell entered listen loop |
| `[bell.unknown.reject]` | 987 | ✅ | ✅ slot=12 type_id=0xffff |
| `[kernel.spawn.sexbell]` | 699 | ✅ | ✅ id=10 |
| `[kernel.sexbell.cap]` | 715 | ✅ | ✅ self slot=12 |

### Negative Checks

| Check | Expected | Found |
|-------|----------|-------|
| Fault/panic | None | ✅ Clean |
| OP_BELL_* parsing | None | ✅ Only 0xFFFF received, no OP_BELL match |
| Queue allocation | None | ✅ No queue code in sexbell |
| Storage calls | None | ✅ No sexstore SLOT in sexbell |
| sexdisplay calls | None | ✅ No display SLOT in sexbell |
| SilkBar calls | None | ✅ No SilkBar SLOT in sexbell |
| Private payload | None | ✅ args=0, no content |

---

## Boot Verification

| PD | Domain | Boots? |
|----|--------|--------|
| sexdisplay | 1 | ✅ |
| sexdrive | 2 | ✅ |
| silk-shell | 3 | ✅ |
| sexinput | 4 | ✅ |
| sexusb | 5 | ✅ |
| silkbar | 6 | ✅ |
| linen | 7 | ✅ |
| sexstore | 8 | ✅ |
| quil | 9 | ✅ |
| sexbell | 10 | ✅ |

Total: 10 PDs. All spawn cleanly. No regressions.

---

## Files Changed

| File | Change | Type |
|------|--------|------|
| `kernel/src/init.rs` | Added test message enqueue after Bell self-cap grant (12 lines, self-contained) | Code |
| `docs/handoff/BELL_UNKNOWN_REJECT_PROOF_V1.md` | New handoff doc | Doc |

### Code Added (init.rs)

```rust
// Bell unknown-reject proof: send one controlled test PDX message with unknown opcode.
if sexbell_id != 0 {
    use crate::ipc::DOMAIN_REGISTRY;
    use crate::ipc::messages::MessageType;
    if let Some(pd) = DOMAIN_REGISTRY.get(sexbell_id) {
        let test_msg = MessageType::IpcCall {
            func_id: 0xFFFF,
            arg0: 0, arg1: 0, arg2: 0,
            caller_pd: 0,
        };
        unsafe { let _ = (*pd.message_ring).enqueue(test_msg); }
        serial_println!("[kernel.sexbell.test] op=0xFFFF slot={}", sex_pdx::SLOT_BELL);
    }
}
```

---

## Scope Confirmation

| Area | Touched? | Evidence |
|------|----------|----------|
| sex-pdx constants | ❌ | SLOT_BELL=12, OP_BELL_*=0xC0-0xC7 unchanged |
| kernel cap grants | ❌ | No new caps — kernel enqueues directly |
| silk-shell | ❌ | Not edited |
| sexbell protocol parsing | ❌ | Stub still only emits `[bell.unknown.reject]` |
| BellEvent/queue | ❌ | Not implemented |
| sexdisplay | ❌ | Not touched |
| sexstore | ❌ | Not touched |
| SilkBar | ❌ | Not touched |
| limine.cfg | ❌ | Already correct |

---

## STOP FIRST Findings

| # | Condition | Check | Stop? |
|---|-----------|-------|-------|
| S1 | Proof requires broad cap grants | ✅ Kernel enqueues directly — no cap grant needed | ❌ Not triggered |
| S2 | Proof requires kernel ABI changes | ✅ Only enqueue existing MessageType::IpcCall | ❌ Not triggered |
| S3 | Proof requires Bell protocol parsing | ✅ 0xFFFF is unknown, falls to `_` arm | ❌ Not triggered |
| S4 | Proof requires sender identity redesign | ✅ caller_pd=0 (kernel) | ❌ Not triggered |
| S5 | Proof requires sexdisplay/SilkBar/storage edits | ✅ None touched | ❌ Not triggered |
| S6 | Proof causes boot regression | ✅ All 10 PDs spawn | ❌ Not triggered |

**All STOP FIRST conditions pass.**

---

## Final Verdict

```
╔══════════════════════════════════════════════════════╗
║      BELL UNKNOWN REJECT PROOF — PASS               ║
╠══════════════════════════════════════════════════════╣
║ Test message sent:          ✅ op=0xFFFF via kernel ║
║ sexbell received:           ✅ pdx_listen dequeued  ║
║ [bell.unknown.reject]:      ✅ slot=12 type=0xffff  ║
║ No OP_BELL_* parsing:       ✅                      ║
║ No fault/panic:             ✅                      ║
║ No cap grants added:        ✅                      ║
║ No side effects:            ✅                      ║
║ All 10 PDs boot:            ✅                      ║
╚══════════════════════════════════════════════════════╝
```

---

## Next Phase Recommendation

**BELL_NOTIFY_PROOF_V1** — Add a controlled `OP_BELL_NOTIFY` (0xC0) test call from an approved sender (e.g., silk-shell) to sexbell, proving the notify route works. This would be the first Bell protocol crossing and requires granting a sender cap to the test PD.

---

## References

- `BELL_SPAWN_PROOF_V1.md` — boot proof (sexbell spawns at domain 10)
- `BELL_BOOT_SPAWN_V1.md` — kernel spawn implementation
- `BELL_SLOT_OPCODE_ASSIGNMENT_V1.md` — SLOT_BELL=12, OP_BELL_* 0xC0-0xC7
- `kernel/src/init.rs` — test message enqueue (line ~179)
- `servers/sexbell/src/main.rs` — unknown reject handler
- `/home/xirtus_arch/Documents/microkernel/logs/qemu-latest.log` — raw boot log

---

*End of BELL_UNKNOWN_REJECT_PROOF_V1.md*
