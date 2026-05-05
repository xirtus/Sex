# BELL_RAM_QUEUE_PLAN_V1

**Status:** Docs-only plan. No code changed.
**Build:** N/A (no code changes).
**Date:** 2026-05-05
**Depends on:** `BELL_PHASE1_FREEZE_V1.md`

---

## 1. Purpose

Design the smallest bounded no_std RAM queue for sexbell to store accepted OP_BELL_NOTIFY events. Currently, sexbell validates and emits markers but discards the event immediately. A queue is required before any downstream consumer (list API, SilkBar presence, inbox rendering) can be designed.

**No implementation.** This plan only. STOP FIRST gates apply before any code change.

---

## 2. Queue Shape

### Data structure

Fixed-size static array with head/tail cursor. No heap, no Vec, no linked list, no dynamic allocation.

```rust
/// Bounded FIFO ring buffer for accepted Bell events.
/// No heap. No dynamic allocation. No private content.
#[repr(C)]
struct BellQueue {
    /// Monotonic event ID counter. Wraps on overflow; uniqueness
    /// is not guaranteed across wrap but is sufficient for V1.
    next_event_id: u64,

    /// Head index (oldest entry).
    head: u16,

    /// Tail index (next write position).
    tail: u16,

    /// Current entry count (avoids wrap-ambiguous head==tail).
    count: u16,

    /// Fixed-size entry array.
    entries: [BellQueueEntry; QUEUE_CAPACITY],
}
```

### Recommendation: `QUEUE_CAPACITY = 16`

| Capacity | Rationale |
|----------|-----------|
| **16** | Fits in ~1KB (16 × ~60 bytes). Sufficient for V1 proof. Overflows are detectable in QEMU proof without requiring many events. Easy to raise to 32 or 64 later. |
| 32 | ~2KB. Reasonable but overkill for V1 proof of concept. |
| 64 | ~4KB. Matches original event model design doc recommendation. Future capacity after queue is proven. |

**Decision: 16 for V1.** Raisable to 32 or 64 by changing a single constant.

### Total size

```
8 (next_event_id) + 2 (head) + 2 (tail) + 2 (count) + 2 (padding) + 16 * entry_size
```

With `entry_size ≈ 40` bytes → `~656 bytes` total. Well within no_std, no-heap constraints.

---

## 3. Event Entry Fields

Only StructuralMeta-safe numeric fields are stored. No title, body, strings, hashes, or private payloads.

### `BellQueueEntry` struct

```rust
/// Single queue entry for an accepted Bell notification.
/// StructuralMeta-only fields. No private content.
#[repr(C)]
struct BellQueueEntry {
    /// Monotonic event ID assigned by Bell on accept.
    event_id:        u64,

    /// Kernel-authoritative sender PD (from msg.caller_pd).
    caller_pd:       u32,

    /// Event category (BellCategory enum).
    category:        u8,

    /// Urgency hint from sender (0..3).
    requested_lane:  u8,

    /// Final lane after policy derivation (0..5).
    final_lane:      u8,

    /// Final urgency after policy derivation (0..3).
    final_urgency:   u8,

    /// Privacy level (BellPrivacyLevel enum 0..3).
    privacy_level:   u8,

    /// Redaction class (BellRedactionClass enum 0..3).
    redaction_class: u8,

    /// 0 for first V1 (action_count always 0).
    action_count:    u8,

    /// 0 for first V1 (object_refs always 0).
    object_ref_count: u8,

    /// Padding to natural alignment.
    _pad:            [u8; 6],
}
```

### Field selection rationale

| Field | Included? | Reason |
|-------|-----------|--------|
| `event_id` | ✅ Yes | Required for close/action/dismiss by ID |
| `caller_pd` | ✅ Yes | Required for cap lookup and sender identification |
| `category` | ✅ Yes | StructuralMeta — safe to store |
| `requested_lane` | ✅ Yes | StructuralMeta — useful for debug/downgrade audit |
| `final_lane` | ✅ Yes | StructuralMeta — canonical lane for filtering/rendering |
| `final_urgency` | ✅ Yes | StructuralMeta — derived urgency |
| `privacy_level` | ✅ Yes | StructuralMeta — needed for render policy |
| `redaction_class` | ✅ Yes | StructuralMeta — needed for proof logging |
| `action_count` | ✅ Yes | Always 0 in V1; reserved for future |
| `object_ref_count` | ✅ Yes | Always 0 in V1; reserved for future |
| `sender_identity_token` | ❌ No | SenderMeta — not needed until cap table exists |
| `trust_label` | ❌ No | SenderMeta — not needed until cap table exists |
| `workspace_context` | ❌ No | Not provided by kernel sender; add when silk-shell provides context |
| `scene_context` | ❌ No | Same as workspace_context |
| `expires_at_ticks` | ❌ No | No expiry mechanism yet; add when queue lifecycle is implemented |
| `action_caps[]` | ❌ No | SecretContent — never stored |
| `object_refs[]` | ❌ No | SecretContent — never stored |
| Title/body | ❌ No | SecretContent — never stored |

### Entry size

```
8 + 4 + 1+1+1+1+1+1+1+1 + 6 = 25 bytes → padded to 32 bytes
```

**Total queue: `16 × 32 = 512 bytes` + 14 bytes header ≈ 526 bytes.** Fits in two cache lines for header + under one 4K page for entries.

---

## 4. Notify Handler Changes (when implemented)

### Current flow (Phase 1)

```
Parse fields → Validate enums → Derive lane → Emit markers → loop
```

### Proposed flow (Phase 2 with queue)

```
Parse fields → Validate enums → Derive lane → Emit recv/downgrade markers
                                  ↓
                      ┌─ Ring buffer full?
                      ├── Yes → Emit [bell.queue.reject.full] → Emit ok (no entry) → loop
                      └── No  → Push entry → Emit [bell.queue.push]
                                → Emit ok marker → loop
```

### Key decisions

1. **Validation and derivation happen before queue push** — no invalid entries enter the queue.
2. **Full queue does NOT overwrite** — sender gets a FULL status (deferred — no reply path yet) and the event is dropped with `[bell.queue.reject.full]` marker.
3. **event_id is assigned at queue push time** — after validation passes. This ensures no event_id gap on rejected messages.
4. **Event order is FIFO** — oldest at head, newest at tail.

### Pseudocode

```rust
OP_BELL_NOTIFY => {
    // 1. Parse + validate (same as Phase 1)
    // 2. Derive lane (same as Phase 1)
    // 3. Emit recv/downgrade markers (same as Phase 1)
    // 4. Try push to queue
    if queue.count >= QUEUE_CAPACITY {
        // Queue full — drop event
        emit "[bell.queue.reject.full]";
        // Future: reply with FULL status
    } else {
        let event_id = queue.next_event_id;
        queue.next_event_id += 1;
        queue.push(BellQueueEntry {
            event_id,
            caller_pd,
            category,
            requested_lane: urgency_hint,
            final_lane,
            final_urgency,
            privacy_level,
            redaction_class,
            action_count: 0,
            object_ref_count: 0,
            _pad: [0; 6],
        });
        emit "[bell.queue.push] event_id={} final_lane={}";
    }
    // 5. Emit ok marker (same as Phase 1)
}
```

---

## 5. Overflow Policy: Reject-When-Full (Hard Limit)

### Policy

| Condition | Action | Marker |
|-----------|--------|--------|
| `count < QUEUE_CAPACITY` | Push entry, increment count | `[bell.queue.push]` |
| `count >= QUEUE_CAPACITY` | Drop event, emit marker | `[bell.queue.reject.full]` |

### Rationale

| Alternative | Rejected because |
|-------------|------------------|
| **Overwrite oldest passive** | Silent data loss — sender thinks event was stored but it was overwritten |
| **Overwrite oldest regardless of lane** | Could lose URGENT/PERSISTENT events — violates urgency semantics |
| **Grow dynamically** | Needs heap — violates no_std constraint |
| **Evict lowest-priority** | Additional complexity for V1; requires lane comparison logic |

**Hard reject is the safest V1 policy.** A future version may implement lane-aware eviction (never drop URGENT/PERSISTENT; drop PASSIVE first) but that is explicitly out of scope for V1.

---

## 6. Event ID Assignment

```rust
/// Monotonic counter. Assigned at queue push time.
/// Wraps from u64::MAX to 1 (0 reserved for "no event").
fn assign_event_id(counter: &mut u64) -> u64 {
    let id = *counter;
    *counter = counter.wrapping_add(1);
    if id == u64::MAX {
        *counter = 1;  // skip 0 on wrap
    }
    id
}
```

- `event_id = 0` is reserved as "invalid/no event" sentinel
- Monotonic (not necessarily consecutive — gaps from rejected messages are fine)
- Wrap is safe for V1 (u64 wraparound at ~10^19 events — not a practical concern)

---

## 7. Markers (Added)

| Marker | Budget | Fields | When |
|--------|--------|--------|------|
| `[bell.queue.push]` | 64 | `event_id`, `final_lane` | Successful queue push |
| `[bell.queue.reject.full]` | 16 | — | Queue at capacity, event dropped |

### Markers that remain unchanged

| Marker | Budget | Unchanged? |
|--------|--------|-----------|
| `[bell.notify.recv]` | 8 | ✅ Same as Phase 1 (emitted before queue push) |
| `[bell.notify.downgrade]` | 8 | ✅ Same as Phase 1 (emitted before queue push) |
| `[bell.notify.ok]` | 8 | ✅ Same as Phase 1 (emitted after queue push/reject) |
| `[bell.notify.reject]` | 4 | ✅ Same as Phase 1 (validation failure only) |
| `[bell.unknown.reject]` | 8 | ✅ Same as Phase 1 (unmatched type_id) |

### Expected boot log (single valid notify → queue)

```
[bell.boot]
[bell.notify.recv] caller_pd=0 category=0 requested=2
[bell.notify.downgrade] from=2 to=0 reason=no_caps_untrusted
[bell.queue.push] event_id=1 final_lane=0
[bell.notify.ok] caller_pd=0 final_lane=0
```

---

## 8. List/Read API Decision: Deferred

### Decision

**OP_BELL_LIST is deferred to a separate phase (`BELL_LIST_SUMMARY_PLAN_V1`).**

### Rationale

| Reason | Detail |
|--------|--------|
| Queue must exist first | List API reads from the queue — can't design read before write |
| Wire format needs its own spec | `BellListRequest` / `BellListReply` / `BellEventSummary` structs need careful review |
| Reply path not yet implemented | sexbell currently has no reply path — list requires pdx_reply or pdx_call_checked |
| scope isolation | Mixing queue + list in one change violates "one thing at a time" |

### What is deferred

| Feature | Deferred to |
|---------|-------------|
| `OP_BELL_LIST` handler | `BELL_LIST_SUMMARY_PLAN_V1` |
| `BellEventSummary` wire format | `BELL_LIST_SUMMARY_PLAN_V1` |
| Reply path implementation | `BELL_LIST_SUMMARY_PLAN_V1` |
| SilkBar presence (lane counts) | `BELL_SILKBAR_PRESENCE_V1` (after list API) |
| Inbox rendering | `BELL_INBOX_ROWS_V1` (after SilkBar presence) |

---

## 9. STOP FIRST Gates

**STOP FIRST** before any of the following:

1. Queue needs heap, Vec, or dynamic allocation.
2. Private title/body/string/hash fields appear in queue entries.
3. Storage or persistence is added (sexstore integration is a separate gate).
4. SilkBar or list API is mixed into the same implementation patch.
5. Real sender caps or external notify grants are mixed into the same patch.
6. ABI or sex-pdx edits are required (no OP_BELL_LIST, no new opcodes).
7. sexdisplay or renderer ownership is touched.
8. Overflow policy is undefined or ambiguous.
9. Queue capacity exceeds 64 entries without a design review.
10. Event ID provenance is unclear (must be assigned by Bell at push time, not by sender).

---

## 10. Proof Plan

### Proof scaffold (same pattern as Phase 1)

A single kernel one-shot OP_BELL_NOTIFY to sexbell. After the notify handler runs:
- `[bell.queue.push] event_id=1 final_lane=0` appears
- One entry exists in the queue
- No `[bell.queue.reject.full]`

### Overflow proof (deferred)

Proving queue-full rejection requires either:
- Sending QUEUE_CAPACITY + 1 notify messages (requires a loop in kernel or a real sender)
- Or temporarily reducing QUEUE_CAPACITY to 1 for a single proof

Deferred to `BELL_RAM_QUEUE_PROOF_V1` or `BELL_RAM_QUEUE_OVERFLOW_PROOF_V1`.

### Cleanup

Kernel proof scaffolding removed after proof verification.

---

## 11. Implementation Plan (for BELL_RAM_QUEUE_IMPLEMENT_V1)

| Step | File | Change |
|------|------|--------|
| 1 | `servers/sexbell/src/main.rs` | Add `BellQueueEntry` struct |
| 2 | `servers/sexbell/src/main.rs` | Add `BellQueue` struct with head/tail/count/entries |
| 3 | `servers/sexbell/src/main.rs` | Add `QUEUE_CAPACITY` constant (= 16) |
| 4 | `servers/sexbell/src/main.rs` | Add `static mut QUEUE: BellQueue` initialization |
| 5 | `servers/sexbell/src/main.rs` | Add `queue_push()` and `queue_is_full()` methods |
| 6 | `servers/sexbell/src/main.rs` | Integrate queue push into OP_BELL_NOTIFY handler after validation |
| 7 | `servers/sexbell/src/main.rs` | Add `[bell.queue.push]` and `[bell.queue.reject.full]` markers |
| 8 | `servers/sexbell/src/main.rs` | Update ok marker to include `event_id` |
| 9 | `kernel/src/init.rs` | Add temporary one-shot scaffolding (removed after proof) |
| 10 | — | Build, boot, proof |

### Non-targets (explicitly excluded)

- No OP_BELL_LIST implementation
- No reply path
- No SilkBar integration
- No sender caps
- No storage
- No private content
- No action callbacks
- No sound
- No overflow proof (deferred)

---

## 12. Next Phases (Recommended Order)

| Phase | Scope | Type |
|-------|-------|------|
| **BELL_RAM_QUEUE_IMPLEMENT_V1** | Add BellQueue struct + push to notify handler | Implementation |
| **BELL_RAM_QUEUE_PROOF_V1** | QEMU boot proof showing `[bell.queue.push]` | Proof |
| **BELL_RAM_QUEUE_CLEANUP_V1** | Remove kernel scaffold | Cleanup |
| **BELL_LIST_SUMMARY_PLAN_V1** | Design OP_BELL_LIST reply wire format | Docs |
| **BELL_LIST_SUMMARY_IMPLEMENT_V1** | Implement OP_BELL_LIST with summary-only replies | Implementation |
| **BELL_SENDER_CAP_PLAN_V1** | Design real sender cap path (BellCap table) | Docs |
| **BELL_SILKBAR_PRESENCE_V1** | Wire lane-count summary to SilkBar | Implementation |
| **BELL_INBOX_ROWS_V1** | Full inbox surface | Implementation |

---

## References

- `BELL_PHASE1_FREEZE_V1.md` — current frozen state of sexbell
- `BELL_EVENT_MODEL_DESIGN_GATE_V1.md` — BellEvent struct (66 bytes) definition
- `BELL_PDX_PROTOCOL_SPEC_V1.md` — BellNotifyRequest/BellNotifyReply message shapes
- `servers/sexbell/src/main.rs` — current handler with validation + lane derivation
- `crates/sex-pdx/src/lib.rs` — opcode and slot constants

---

*End of BELL_RAM_QUEUE_PLAN_V1.md*
