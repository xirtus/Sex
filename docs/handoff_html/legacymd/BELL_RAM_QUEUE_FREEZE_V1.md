# BELL_RAM_QUEUE_FREEZE_V1

**Status:** Bell Phase 2 RAM queue complete. Frozen.
**Build:** `[SEXOS ENTRYPOINT] success`
**Date:** 2026-05-05
**Depends on:** `BELL_PHASE1_FREEZE_V1.md`, `BELL_RAM_QUEUE_CLEANUP_V1.md`

---

## 1. Final Queue State

sexbell now stores accepted OP_BELL_NOTIFY events in a bounded 16-entry RAM queue with monotonic event IDs and reject-when-full overflow policy.

### Queue struct

```rust
const BELL_QUEUE_CAPACITY: usize = 16;

struct BellQueue {
    next_event_id: u64,                     // monotonic counter, starts at 1
    head: u16,                              // oldest entry index
    tail: u16,                              // next write index
    count: u16,                             // current entry count
    entries: [BellQueueEntry; 16],          // fixed-size array
}
```

### Entry fields

| Field | Type | StructuralMeta? |
|-------|------|----------------|
| `event_id` | u64 | ✅ |
| `caller_pd` | u32 | ✅ |
| `category` | u8 | ✅ |
| `requested_lane` | u8 | ✅ |
| `final_lane` | u8 | ✅ |
| `final_urgency` | u8 | ✅ |
| `privacy_level` | u8 | ✅ |
| `redaction_class` | u8 | ✅ |
| `action_count` | u8 | ✅ (always 0) |
| `object_ref_count` | u8 | ✅ (always 0) |

**Not stored:** sender_identity_token, trust_label, workspace_context, scene_context, expires_at_ticks, action_caps, object_refs, title, body, sender name, file paths, or any private content.

### Size: ~526 bytes (no heap)

| Component | Size |
|-----------|------|
| Header (next_event_id + head + tail + count) | 14 bytes |
| Entry array (16 × 32 bytes) | 512 bytes |
| **Total** | **~526 bytes** |

### Storage: `static mut BELL_QUEUE: BellQueue = BellQueue::new();`

No heap, no Vec, no String, no Box, no dynamic allocation.

---

## 2. Handler Flow (Final)

```
Receive message
  │
  ├── type_id == OP_BELL_NOTIFY (0xC0)
  │     ├── Parse fields from arg0/arg1/arg2
  │     ├── Validate enums ──→ fail → [bell.notify.reject] → continue
  │     ├── [bell.notify.recv]
  │     ├── Derive lane (no caps → PASSIVE) → [bell.notify.downgrade] (if applicable)
  │     ├── BellQueue.push()
  │     │     ├── Ok(event_id) → [bell.queue.push] → [bell.notify.ok] → continue
  │     │     └── Err(full)    → [bell.queue.reject.full] → [bell.notify.reject] → continue
  │
  └── type_id != OP_BELL_NOTIFY
        └── [bell.unknown.reject] → continue
```

---

## 3. Proof History (Bell All Phases)

| # | Phase | Handoff | Result |
|---|-------|---------|--------|
| 1 | Event model design | `BELL_EVENT_MODEL_DESIGN_GATE_V1.md` | ✅ Docs |
| 2 | Cap policy | `BELL_CAPABILITY_POLICY_V1.md` | ✅ Docs |
| 3 | Protocol spec | `BELL_PDX_PROTOCOL_SPEC_V1.md` | ✅ Docs |
| 4 | Namespace audit | `BELL_NAMESPACE_COLLISION_AUDIT_V1.md` | ✅ Docs |
| 5 | Slot/opcode assignment | `BELL_SLOT_OPCODE_ASSIGNMENT_V1.md` | ✅ sex-pdx constants |
| 6 | Server stub plan | `BELL_SERVER_STUB_PLAN_V1.md` | ✅ Docs |
| 7 | Server stub | `BELL_SERVER_STUB_V1.md` | ✅ Crate |
| 8 | Boot spawn plan | `BELL_BOOT_SPAWN_PLAN_V1.md` | ✅ Docs |
| 9 | Boot spawn | `BELL_BOOT_SPAWN_V1.md` | ✅ Kernel spawn |
| 10 | Spawn proof | `BELL_SPAWN_PROOF_V1.md` | ✅ QEMU |
| 11 | Unknown reject proof | `BELL_UNKNOWN_REJECT_PROOF_V1.md` | ✅ QEMU |
| 12 | Unknown reject cleanup | `BELL_UNKNOWN_REJECT_CLEANUP_V1.md` | ✅ Clean |
| 13 | Notify plan | `BELL_NOTIFY_PLAN_V1.md` | ✅ Docs |
| 14 | Notify implement | `BELL_NOTIFY_IMPLEMENT_V1.md` | ✅ Handler |
| 15 | Notify proof | `BELL_NOTIFY_PROOF_V1.md` | ✅ QEMU |
| 16 | Notify cleanup | `BELL_NOTIFY_CLEANUP_V1.md` | ✅ Clean |
| 17 | Negative plan | `BELL_NOTIFY_NEGATIVE_PROOF_PLAN_V1.md` | ✅ Docs |
| 18 | Negative proof | `BELL_NOTIFY_NEGATIVE_PROOF_V1.md` | ✅ QEMU |
| 19 | Negative cleanup | `BELL_NOTIFY_NEGATIVE_CLEANUP_V1.md` | ✅ Clean |
| 20 | Phase 1 freeze | `BELL_PHASE1_FREEZE_V1.md` | ✅ Frozen |
| 21 | RAM queue plan | `BELL_RAM_QUEUE_PLAN_V1.md` | ✅ Docs |
| 22 | RAM queue implement | `BELL_RAM_QUEUE_IMPLEMENT_V1.md` | ✅ Queue |
| 23 | RAM queue proof | `BELL_RAM_QUEUE_PROOF_V1.md` | ✅ QEMU |
| 24 | RAM queue cleanup | `BELL_RAM_QUEUE_CLEANUP_V1.md` | ✅ Clean |
| **25** | **RAM queue freeze** | **`BELL_RAM_QUEUE_FREEZE_V1.md`** | **✅ Here** |

---

## 4. Scaffold Absence Confirmation

All temporary kernel test enqueues have been removed across all Bell phases:

| Scaffold | Phase | Verified |
|----------|-------|----------|
| `0xFFFF` IpcCall test | Unknown reject cleanup | ✅ |
| `[kernel.sexbell.notify.test]` | Notify cleanup | ✅ |
| `[kernel.sexbell.notify.invalid.test]` | Negative cleanup | ✅ |
| `[kernel.sexbell.queue.test]` | RAM queue cleanup | ✅ |

```bash
rg -n "kernel.sexbell" kernel/src/init.rs
# → only [kernel.sexbell.cap] and [kernel.spawn.sexbell] remain
```

---

## 5. Forbidden Features — Confirmed Absent

| Feature | Check | Result |
|---------|-------|--------|
| OP_BELL_LIST handler | `rg "OP_BELL_LIST" sexbell/main.rs` | ❌ Absent (reserved in sex-pdx only) |
| Reply path | `rg "pdx_reply\|pdx_call" sexbell/main.rs` | ❌ Absent |
| Heap/alloc/Vec/String/Box | `rg "Vec\|String\|alloc\|Box\|heap" sexbell/main.rs` | ❌ Absent |
| Sender caps (external SLOT_BELL) | `rg "SLOT_BELL.*Domain" init.rs` | ❌ Only self-cap |
| SilkBar integration | `rg "silkbar" sexbell/main.rs` | ❌ Absent |
| Rendering/sexdisplay | `rg "sexdisplay\|0xEC\|0xEF" sexbell/main.rs` | ❌ Absent |
| Storage/persistence | `rg "store\|persist" sexbell/main.rs` | ❌ Absent |
| Private text/title/body | `rg "title\|body\|sender_name" sexbell/main.rs` | ❌ Absent |
| Action callbacks | `rg "action_callback\|action.*dispatch" sexbell/main.rs` | ❌ Absent |
| Sound/audio | `rg "sound\|audio\|harp" sexbell/main.rs` | ❌ Absent |
| Kernel notify sender | `rg "MessageType::IpcCall.*OP_BELL_NOTIFY" init.rs` | ❌ No enqueue |

---

## 6. sex-pdx Constants (Unchanged)

| Constant | Value | Status |
|----------|-------|--------|
| `SLOT_BELL` | 12 | ✅ Final (Phase 1) |
| `OP_BELL_NOTIFY` | 0xC0 | ✅ Final (Phase 1) |
| `OP_BELL_CLOSE` | 0xC1 | ✅ Reserved, unused |
| `OP_BELL_ACTION` | 0xC2 | ✅ Reserved, unused |
| `OP_BELL_LIST` | 0xC3 | ✅ Reserved, unused (next phase) |
| `OP_BELL_CLEAR` | 0xC4 | ✅ Reserved, unused |
| `OP_BELL_SUBSCRIBE` | 0xC5 | ✅ Reserved, unused |
| `OP_BELL_SET_POLICY` | 0xC6 | ✅ Reserved, unused |
| `OP_BELL_MUTE_SENDER` | 0xC7 | ✅ Reserved, unused |

---

## 7. Known Limitations

| Limitation | Impact | Next Phase |
|------------|--------|------------|
| No active sender | sexbell queues nothing at runtime | Design sender cap path |
| No list/read API | No way to inspect queued events | **BELL_LIST_SUMMARY_PLAN_V1** |
| No queue-full runtime proof | Overflow behavior untested in QEMU | Deferred overflow proof |
| No BellCap sender policy | All senders treated as untrusted (PASSIVE) | BellCap table phase |
| No SilkBar presence | No visual indicator of event count | After list API |
| No storage/persistence | Events lost on reboot | E-series storage gate |
| No private content transport | Title/body not on wire | Content-token design gate |
| No action callbacks | Dismiss/action not wired | Action dispatch phase |

---

## 8. Phase 2 Verdict

**Bell Phase 2 (RAM queue) is frozen.**

| Component | Status |
|-----------|--------|
| Queue design | ✅ Complete |
| Queue implementation | ✅ Complete (16-entry, no heap) |
| Queue proof | ✅ Complete (event_id=1, count=1) |
| Scaffolds | ✅ All removed |
| sex-pdx edits | ✅ None needed (OP_BELL_NOTIFY=0xC0 already assigned) |
| Kernel edits | ✅ None beyond temporary scaffold (removed) |

---

## 9. Next Recommended Phase

**BELL_LIST_SUMMARY_PLAN_V1** — Design the OP_BELL_LIST wire format for reading queued event summaries. This requires:
- `BellEventSummary` struct (StructuralMeta-only, no private content)
- `BellListRequest` / `BellListReply` message shapes
- Reply path (sexbell must respond to pdx_call)
- Summary-only: no full event bodies, no private fields

After list API: sender cap path → SilkBar presence → inbox rendering.

---

## References

- All 24 prior Bell handoff documents in `docs/handoff/BELL_*.md`
- `servers/sexbell/src/main.rs` — queue implementation + handler
- `kernel/src/init.rs` — spawn + self-cap (no enqueues)
- `crates/sex-pdx/src/lib.rs` — constants

---

*End of BELL_RAM_QUEUE_FREEZE_V1.md*
