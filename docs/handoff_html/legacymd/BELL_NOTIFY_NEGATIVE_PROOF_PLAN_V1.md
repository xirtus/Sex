# BELL_NOTIFY_NEGATIVE_PROOF_PLAN_V1

**Status:** Docs-only plan. No code changed.
**Build:** N/A (no code changes).
**Date:** 2026-05-05
**Depends on:** `BELL_NOTIFY_CLEANUP_V1.md`

---

## 1. Purpose

Plan one controlled invalid OP_BELL_NOTIFY payload that proves `[bell.notify.reject]` fires for bad enums. Only one negative test. No broad negative test suite. After this proof, Bell Phase 1 freezes.

**No implementation.** This plan only. STOP FIRST gates apply before any code change.

---

## 2. Negative Test: Invalid Category

### Why category?

The validation chain in sexbell is priority-ordered:

```
1. valid_category      → invalid_category   (first check)
2. valid_privacy_level → invalid_privacy
3. valid_redaction_class → invalid_redaction
4. urgency_hint > 3    → invalid_urgency
5. action_count != 0   → action_count_not_zero
6. object_refs != 0    → object_refs_not_zero
```

Category is the **first** check. Setting `category = 7` (out of range 0..=5) triggers the earliest possible rejection. This is the simplest, most direct negative test.

### Rejected alternatives

| Alternative | Reason rejected |
|-------------|-----------------|
| `privacy_level = 7` | Works but tests same code path (enum range check) — no advantage over category |
| `urgency_hint = 7` | Would test a different branch but category is checked first so 7 would be caught before urgency regardless |
| `action_count = 1` | Would require the enum checks to pass first — less isolated test |
| `object_refs = 1` | Same as action_count — requires valid enums to reach that check |

Category=7 is the **minimum invalid change** that guarantees rejection at the earliest possible point.

---

## 3. Sender Decision

Same as the valid proof: **kernel one-shot direct message, removed after proof**.

Rationale:
- No permanent cap grants needed
- No silk-shell or app edits
- Only changes `arg0` field value — everything else is structurally identical to the valid proof scaffolding
- Single 18-line block in init.rs, removed immediately after proof

The kernel scaffolding uses the same pattern as BELL_NOTIFY_IMPLEMENT_V1:

```rust
if sexbell_id != 0 {
    use crate::ipc::{DOMAIN_REGISTRY, messages::MessageType};
    // Pack fields: category=7(INVALID), urgency_hint=URGENT(2),
    //              privacy_level=Public(0), redaction_class=StructuralMeta(0)
    let arg0: u64 = (7u64 << 0)  | (2u64 << 8)  | (0u64 << 16) | (0u64 << 24);
    let msg = MessageType::IpcCall {
        func_id:   sex_pdx::OP_BELL_NOTIFY,
        arg0,
        arg1:      0,
        arg2:      0,
        caller_pd: 0,
    };
    if let Some(pd) = DOMAIN_REGISTRY.get(sexbell_id) {
        unsafe { let _ = (*pd.message_ring).enqueue(msg); }
        serial_println!("[kernel.sexbell.notify.invalid.test] category=7");
    }
}
```

---

## 4. Exact Payload Packing

### arg0 bit layout

| Bits | Field | Value | Meaning |
|------|-------|-------|---------|
| 0-7 | `category` | **7** | **INVALID** — outside 0..=5 range |
| 8-15 | `urgency_hint` | 2 | URGENT (valid, never reached) |
| 16-23 | `privacy_level` | 0 | Public (valid, never reached) |
| 24-31 | `redaction_class` | 0 | StructuralMeta (valid, never reached) |
| 32-63 | `_reserved` | 0 | Zero |

### arg1, arg2

| Arg | Value | Meaning |
|-----|-------|---------|
| `arg1` | 0 | action_count = 0 |
| `arg2` | 0 | object_refs = 0 |
| `caller_pd` | 0 | Kernel-originated |

### Why category=7 specifically?

- `valid_category()` = `v <= 5`
- `7 > 5` → returns false
- Reject reason: `"invalid_category"`
- No other checks are reached (else-if chain)

Could also use category=255 (u8::MAX) or any value ≥ 6. Category=7 is the smallest invalid value (next after 5), chosen for clarity.

---

## 5. Expected Markers

### Success path (negative test passes)

```
[kernel.sexbell.notify.invalid.test] category=7
[bell.notify.reject] caller_pd=0 reason=invalid_category
```

### Markers that must be ABSENT

| Marker | Must be absent because |
|--------|----------------------|
| `[bell.notify.recv]` | Validation fails before recv marker |
| `[bell.notify.downgrade]` | Validation fails before lane derivation |
| `[bell.notify.ok]` | Validation fails — no success path |
| `[bell.unknown.reject]` | OP_BELL_NOTIFY is matched correctly |

### Expected behavior flow

```
1. Kernel enqueues IpcCall (category=7)
2. sexbell matches OP_BELL_NOTIFY
3. sexbell parses fields
4. valid_category(7) → false
5. reject_reason = Some("invalid_category")
6. Emit [bell.notify.reject] caller_pd=0 reason=invalid_category
7. continue (loop back to pdx_listen_raw)
```

---

## 6. Edge Cases That Are NOT Tested (and why)

| Edge case | Not tested because |
|-----------|-------------------|
| Multiple invalid fields | Category is checked first — only the first rejection is visible |
| Boundary values (category=5, category=6) | 5 = Info (valid), 6 = out of range (same as 7). Redundant. |
| All fields invalid | Only first rejection matters — no need for combinatorial test |
| Invalid enum with non-zero action_count | Category check fires first — action_count never reached |
| Large arg0 padding bits | Parse masks to 8-bit fields — padding bits are safely ignored |
| Kernel-sent without valid PD | `if sexbell_id != 0` guard prevents enqueue to nonexistent PD |
| Panic on invalid parse | `valid_category` returns bool — no panic path |

---

## 7. Cleanup Requirement

**BELL_NOTIFY_NEGATIVE_CLEANUP_V1** removes the kernel scaffolding immediately after proof.

Same pattern as the valid proof cleanup:
- Remove the 18-line scaffolding block from `init.rs`
- Keep sexbell handler unchanged
- Verify no `[kernel.sexbell.notify.invalid.test]` remains
- Build passes

---

## 8. STOP FIRST Gates

**STOP FIRST** before any of the following:

1. Adding multiple negative tests (plan is for exactly one).
2. Keeping the kernel scaffold beyond the proof phase.
3. Adding any string/body/title/private field to the payload (payload is all numeric).
4. Changing the rejection logic in sexbell (it already works correctly for invalid enums).
5. Adding queue/storage/render/SilkBar/action/sound support.
6. Adding cap grants for any external sender.
7. Needing sex-pdx or ABI edits.
8. Adding a persistent negative test that remains after cleanup.

---

## 9. Implementation Plan (for BELL_NOTIFY_NEGATIVE_PROOF_V1)

| Step | File | Change |
|------|------|--------|
| 1 | `kernel/src/init.rs` | Add 18-line scaffolding block (category=7, `[kernel.sexbell.notify.invalid.test]`) |
| 2 | — | Build, boot, capture log |
| 3 | — | Verify `[bell.notify.reject]`, no `[bell.notify.ok]`, no faults |
| 4 | `docs/handoff/BELL_NOTIFY_NEGATIVE_PROOF_V1.md` | Proof handoff doc |
| 5 | `kernel/src/init.rs` | Remove scaffolding |
| 6 | `docs/handoff/BELL_NOTIFY_NEGATIVE_CLEANUP_V1.md` | Cleanup handoff doc |

---

## 10. Phase 1 Freeze

After negative proof + cleanup, Bell Phase 1 is frozen:

| Component | Status |
|-----------|--------|
| sexbell crate | Exists, boots, listens |
| SLOT_BELL=12 | Assigned in sex-pdx |
| OP_BELL_NOTIFY=0xC0 | Assigned in sex-pdx |
| Valid notify handler | Implemented and proved |
| Invalid enum reject | Plan exists, proof pending |
| Kernel scaffolding (valid) | Removed |
| Kernel scaffolding (invalid) | Removed after proof |
| Queues/storage/render/SilkBar | Not implemented |
| Action callbacks/sound | Not implemented |

**Phase 2 gates** (future, not yet planned):
- Wire a real sender (silk-shell? kernel event?)
- Add ring buffer
- Add BellCap table
- Add SilkBar presence
- Add inbox rendering

---

## References

- `BELL_NOTIFY_IMPLEMENT_V1.md` — valid notify implementation (pattern to copy for scaffolding)
- `BELL_NOTIFY_PROOF_V1.md` — valid proof (pattern for negative proof)
- `BELL_NOTIFY_CLEANUP_V1.md` — valid cleanup (pattern for negative cleanup)
- `servers/sexbell/src/main.rs` — validation chain at lines 60-74
- `kernel/src/init.rs` — sexbell spawn and self-cap (location for scaffolding)

---

*End of BELL_NOTIFY_NEGATIVE_PROOF_PLAN_V1.md*
