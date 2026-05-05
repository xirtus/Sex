# BELL_LIST_SUMMARY_PLAN_V1

**Status:** Docs-only plan. No code changed.
**Build:** N/A (no code changes).
**Date:** 2026-05-05
**Depends on:** `BELL_RAM_QUEUE_FREEZE_V1.md`, `BELL_PDX_PROTOCOL_SPEC_V1.md`

---

## 1. Purpose

Design the smallest safe OP_BELL_LIST summary API over the existing 16-entry RAM queue. sexbell currently stores accepted events but has no way for any component to read them. A list summary is required before SilkBar presence, inbox rendering, or any downstream consumer can be designed.

**No implementation.** This plan only. STOP FIRST gates apply before any code change.

---

## 2. Request Shape

### Simplified for V1

The protocol spec already proposes `BellListRequest`:

```rust
struct BellListRequest {
    lane_filter: u8,     // 0xFF = all lanes, 0..5 = specific lane
    max_results: u8,     // max entries to return (clamped to reply capacity)
    _reserved: [u8; 6],
}
```

### OP_BELL_LIST (0xC3) — already assigned in sex-pdx

| Opcode | Value | Status |
|--------|-------|--------|
| `OP_BELL_LIST` | 0xC3 | ✅ Already assigned, unused |

### arg0/arg1/arg2 packing

Since `pdx_listen_raw` returns `(type_id, arg0, arg1, arg2, caller_pd)`, the list request can be packed as:

| Arg | Bits | Field | Values |
|-----|------|-------|--------|
| `arg0` | 0-7 | `lane_filter` | 0xFF = all, 0..5 = specific lane |
| `arg0` | 8-15 | `max_results` | 1..=8 (clamped to reply capacity) |
| `arg0` | 16-63 | `_reserved` | 0 |

### Validation

- `lane_filter`: 0xFF (all) or 0..=5 (valid lane). Reject otherwise.
- `max_results`: 0 → reject. > 8 → clamp to 8.

---

## 3. Reply Strategy: Marker-Only for V1

### Decision: Marker-only proof, no reply path implementation.

| Option | Verdict | Rationale |
|--------|---------|-----------|
| **A: Marker-only proof** | **PREFERRED** | sexbell emits `[bell.list.reply]` with summary counts. No real reply path. Same pattern as all prior Bell proofs. |
| B: `pdx_reply(caller_pd)` with encoded value | Rejected for V1 | `pdx_reply` is a signal-only syscall (no data payload). Would require encoding summaries into a single u64, which is too cramped for multiple events. |
| C: `pdx_call_checked` return value | Rejected for V1 | Would require a real sender using `pdx_call` instead of kernel direct enqueue. Adds sender/cap complexity. |
| D: Shared memory ring | Rejected | Violates no-shared-memory constraint. Requires cap/ABI changes. |

### Marker-only flow

```
Kernel one-shot → sexbell listens → matches OP_BELL_LIST
                                   → reads queue state
                                   → emits [bell.list.reply] count={} total={}
                                   → loops
```

No reply is sent to the kernel (kernel one-shot doesn't expect one). Proof markers verify the handler parsed the request and read the queue correctly.

### Why marker-only is sufficient for V1

- Proves the OP_BELL_LIST code path works
- Proves queue iteration + summary extraction
- Proves enum validation for lane_filter/max_results
- No sender/cap/reply ABI changes needed
- Same pattern as all previous Bell proofs
- Real reply path can be added when a real sender (SilkBar) is wired

---

## 4. Summary Fields

### `BellEventSummary` (V1 simplified)

Only fields that exist in the queue entry. No fields the queue doesn't store.

```rust
#[repr(C)]
struct BellEventSummary {
    event_id:         u64,
    caller_pd:        u32,
    category:         u8,
    final_lane:       u8,
    final_urgency:    u8,
    privacy_level:    u8,
    redaction_class:  u8,
    requested_lane:   u8,
    action_count:     u8,
    object_ref_count: u8,
    _pad:             [u8; 2],
}
```

**Size: 24 bytes** per summary.

### Field selection

| Field | In queue? | Include in summary? | Rationale |
|-------|-----------|-------------------|-----------|
| `event_id` | ✅ | ✅ Yes | Required to reference events |
| `caller_pd` | ✅ | ✅ Yes | Identify sender |
| `category` | ✅ | ✅ Yes | StructuralMeta |
| `final_lane` | ✅ | ✅ Yes | Canonical lane |
| `final_urgency` | ✅ | ✅ Yes | StructuralMeta |
| `privacy_level` | ✅ | ✅ Yes | StructuralMeta |
| `redaction_class` | ✅ | ✅ Yes | StructuralMeta |
| `requested_lane` | ✅ | ✅ Yes | Useful for debug, StructuralMeta |
| `action_count` | ✅ | ✅ Yes | Always 0 in V1 |
| `object_ref_count` | ✅ | ✅ Yes | Always 0 in V1 |

### Fields NOT in summary (from protocol spec's BellEventSummary)

| Field | In protocol spec? | Include? | Reason |
|-------|------------------|----------|--------|
| `trust_label` | ✅ Proposed | ❌ No | Not stored in queue (SenderMeta) |
| `workspace_context` | ✅ Proposed | ❌ No | Not stored in queue |
| `scene_context` | ✅ Proposed | ❌ No | Not stored in queue |
| `expires_at_ticks` | ✅ Proposed | ❌ No | Not stored in queue |
| Title/body/sender name | ❌ Not in spec | ❌ No | Never stored |

### Reply capacity

| Capacity | Entries | Total reply size | Rationale |
|----------|---------|-----------------|-----------|
| **4** | Up to 4 summaries | ~96 bytes + header | Fits within conservative PDX bounds. Enough for V1 proof. |
| 8 | Up to 8 summaries | ~192 bytes + header | Protocol spec proposes 8. Deferred — can raise later. |

**Decision: 4 for V1.** Raisable to 8 later by changing a constant.

---

## 5. Caller Policy

### V1 policy: default-deny

| Caller | Allowed? | Rationale |
|--------|----------|-----------|
| Unknown/untrusted | ❌ Denied | Default-deny. No read caps granted. |
| Kernel (direct message) | ✅ For proof only | Same pattern as all previous Bell proofs. Scaffold removed after. |
| silk-shell | ❌ Denied | No SLOT_BELL cap. Would need explicit read-cap grant. |
| SilkBar | ❌ Denied | No SLOT_BELL cap. Requires read-cap plan. |

### V1 enforcement

For the marker-only proof, sexbell matches OP_BELL_LIST but does NOT check caller caps (since the only caller is the kernel scaffold). Cap checking is deferred to `BELL_READER_CAP_PLAN_V1` when a real sender is wired.

### Future cap plan (sketch)

```
BELL_LIST_READER_CAP = new capability (or reuse SLOT_BELL)
Granted at spawn time to specific domains (silk-shell, SilkBar)
Default-deny for all others
```

---

## 6. Queue Read Helper

A new method on `BellQueue`:

```rust
impl BellQueue {
    /// Copy up to `max` event summaries into `out` buffer, starting from newest.
    /// Returns the number of summaries written.
    /// Does NOT mutate the queue.
    fn read_newest(&self, max: usize, out: &mut [BellEventSummary]) -> usize {
        let count = core::cmp::min(max, core::cmp::min(self.count as usize, out.len()));
        // Iterate from newest (tail-1, tail-2, ...) wrapping around the ring buffer
        for i in 0..count {
            let idx = (self.tail as usize + BELL_QUEUE_CAPACITY - 1 - i) % BELL_QUEUE_CAPACITY;
            let entry = &self.entries[idx];
            out[i] = BellEventSummary {
                event_id:        entry.event_id,
                caller_pd:       entry.caller_pd,
                category:        entry.category,
                final_lane:      entry.final_lane,
                final_urgency:   entry.final_urgency,
                privacy_level:   entry.privacy_level,
                redaction_class: entry.redaction_class,
                requested_lane:  entry.requested_lane,
                action_count:    entry.action_count,
                object_ref_count: entry.object_ref_count,
                _pad:            [0; 2],
            };
        }
        count
    }
}
```

The read is newest-first (reverse order), which is most useful for displaying recent events.

---

## 7. Markers

| Marker | Budget | Fields | When |
|--------|--------|--------|------|
| `[bell.list.recv]` | 8 | `lane_filter`, `max_results` | Valid OP_BELL_LIST received |
| `[bell.list.reply]` | 8 | `count`, `total` | Queue read completed |
| `[bell.list.empty]` | 4 | — | Queue has 0 matching events |
| `[bell.list.reject]` | 4 | `reason` | Invalid lane_filter or max_results |
| `[bell.list.redacted]` | 4 | `reason` | Future: when privacy redaction strips entries |

### Expected boot log (single event in queue, list all)

```
[bell.list.recv] lane_filter=ff max_results=4
[bell.list.reply] count=1 total=1
```

### Expected boot log (empty queue)

```
[bell.list.recv] lane_filter=ff max_results=4
[bell.list.empty]
```

### Expected boot log (invalid filter)

```
[bell.list.recv] lane_filter=ff max_results=4
[bell.list.reject] reason=invalid_lane_filter
```

---

## 8. Handler Flow (Proposed)

```
match msg.type_id {
    OP_BELL_LIST => {
        // Parse
        let lane_filter  = (msg.arg0 >> 0)  & 0xFF;
        let max_results  = (msg.arg0 >> 8)  & 0xFF;
        let caller_pd    = msg.caller_pd;

        // Validate
        if lane_filter != 0xFF && lane_filter > 5 {
            → [bell.list.reject] reason=invalid_lane_filter → continue
        }
        if max_results == 0 || max_results > 4 {
            → [bell.list.reject] reason=invalid_max_results → continue
        }

        // Emit recv
        → [bell.list.recv] lane_filter={} max_results={}

        // Read queue
        let summaries = ...;    // stack-local array of 4
        let count = unsafe { BELL_QUEUE.read_newest(max_results as usize, &mut summaries) };

        // Emit reply/empty
        if count == 0 {
            → [bell.list.empty]
        } else {
            → [bell.list.reply] count={} total={}
        }
    }

    OP_BELL_NOTIFY => { /* existing */ }
    _ => { /* unknown reject */ }
}
```

---

## 9. Proof Strategy

### Phase A: Marker-only proof (same pattern as all prior Bell proofs)

1. Kernel sends one OP_BELL_NOTIFY → queue has 1 entry
2. Kernel sends one OP_BELL_LIST with lane_filter=0xFF, max_results=4
3. Verify `[bell.list.recv]` + `[bell.list.reply] count=1 total=1`
4. Clean up both scaffolds

### Phase B: Empty queue proof

1. Kernel sends OP_BELL_LIST first (before any notify)
2. Verify `[bell.list.empty]`
3. Clean up scaffold

### Both scaffolds temporary, removed after proof.

---

## 10. STOP FIRST Gates

**STOP FIRST** before any of the following:

1. OP_BELL_LIST is not already assigned in sex-pdx (✅ 0xC3 assigned).
2. Reply path would require ABI changes to PDX or kernel syscalls.
3. Summary struct contains private content (title, body, sender name, file paths).
4. Implementation wants SilkBar presence in the same patch (must be separate phase).
5. Implementation wants queue mutation or clear (OP_BELL_CLEAR is separate).
6. Heap/alloc/Vec/String is needed for summaries (stack-local array only).
7. Caller identity/read-cap policy is unclear for a real sender.
8. Implementation wants to add a real sender with permanent caps in the same patch.
9. The read helper requires unsafe or mutable access to the queue beyond the existing `static mut` pattern.

---

## 11. Implementation Plan (for BELL_LIST_SUMMARY_IMPLEMENT_V1)

| Step | File | Change |
|------|------|--------|
| 1 | `servers/sexbell/src/main.rs` | Add `BellEventSummary` struct (4 fields trimmed from protocol spec) |
| 2 | `servers/sexbell/src/main.rs` | Add `LIST_REPLY_CAPACITY = 4` constant |
| 3 | `servers/sexbell/src/main.rs` | Add `BellQueue::read_newest()` method |
| 4 | `servers/sexbell/src/main.rs` | Add `OP_BELL_LIST` match arm in handler |
| 5 | `servers/sexbell/src/main.rs` | Add list markers |
| 6 | `kernel/src/init.rs` | Temporary proof scaffold (OP_BELL_LIST, removed after) |

### Non-targets

- No real reply path (`pdx_reply` not called)
- No SilkBar/reader integration
- No queue mutation
- No OP_BELL_CLEAR
- No cap checks for real senders
- No changes to sex-pdx (OP_BELL_LIST=0xC3 already assigned)

---

## 12. Next Phases (Recommended Order)

| Phase | Scope | Type |
|-------|-------|------|
| **BELL_LIST_SUMMARY_IMPLEMENT_V1** | Add OP_BELL_LIST handler + read_newest() + markers | Implementation |
| **BELL_LIST_SUMMARY_PROOF_V1** | QEMU boot proof with queue + list scaffolds | Proof |
| **BELL_LIST_SUMMARY_CLEANUP_V1** | Remove scaffolds | Cleanup |
| **BELL_READER_CAP_PLAN_V1** | Design read-cap policy for real senders (silk-shell, SilkBar) | Docs |
| **BELL_SILKBAR_PRESENCE_PLAN_V1** | Design lane-count summary push to SilkBar | Docs |

---

## References

- `BELL_RAM_QUEUE_FREEZE_V1.md` — current queue state (16-entry, StructuralMeta-only)
- `BELL_PDX_PROTOCOL_SPEC_V1.md` — proposed BellListRequest/BellListReply/BellEventSummary shapes
- `BELL_RAM_QUEUE_IMPLEMENT_V1.md` — queue implementation details
- `servers/sexbell/src/main.rs` — current handler with OP_BELL_NOTIFY
- `crates/sex-pdx/src/lib.rs` — OP_BELL_LIST=0xC3 already assigned

---

*End of BELL_LIST_SUMMARY_PLAN_V1.md*
