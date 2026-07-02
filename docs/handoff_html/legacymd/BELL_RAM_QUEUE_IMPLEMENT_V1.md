# BELL_RAM_QUEUE_IMPLEMENT_V1

**Status:** Implementation complete.
**Build:** `[SEXOS ENTRYPOINT] success`

---

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `servers/sexbell/src/main.rs` | Add RAM queue struct + push integration in OP_BELL_NOTIFY handler | ~100 |

**Not touched:**
- `kernel/src/init.rs` — no changes (no scaffold yet)
- `crates/sex-pdx/src/lib.rs` — no changes
- `limine.cfg` — no changes
- `sexos_build_spec.toml` — no changes
- Any other file

---

## Queue Capacity

**16 entries** — defined as `BELL_QUEUE_CAPACITY: usize = 16`.

### Size calculation

```
Entry: 8+4+1+1+1+1+1+1+1+1+6 (pad) = 32 bytes
Header: 8+2+2+2 = 14 bytes
Total: 14 + 16 × 32 = 526 bytes
```

---

## Entry Fields

| Field | Type | Source | StructuralMeta? |
|-------|------|--------|----------------|
| `event_id` | u64 | Assigned at push time (monotonic) | ✅ |
| `caller_pd` | u32 | `msg.caller_pd` | ✅ |
| `category` | u8 | Parsed from arg0 bits 0-7 | ✅ |
| `requested_lane` | u8 | urgency_hint from arg0 bits 8-15 | ✅ |
| `final_lane` | u8 | Derived by `derive_lane_first_proof()` | ✅ |
| `final_urgency` | u8 | Derived by `derive_lane_first_proof()` | ✅ |
| `privacy_level` | u8 | Parsed from arg0 bits 16-23 | ✅ |
| `redaction_class` | u8 | Parsed from arg0 bits 24-31 | ✅ |
| `action_count` | u8 | Always 0 in V1 | ✅ |
| `object_ref_count` | u8 | Always 0 in V1 | ✅ |

**Not stored:** sender_identity_token, trust_label, workspace_context, scene_context, expires_at_ticks, action_caps, object_refs, title, body, sender name, file paths, or any private content.

---

## Event ID Behavior

- **Monotonic u64 counter** starting at 1 (0 is reserved as invalid sentinel).
- Assigned **at queue push time** — after validation and lane derivation.
- Wrap: if counter reaches `u64::MAX`, next value is 1 (skips 0).
- `event_id` is returned from `BellQueue::push()` and included in `[bell.notify.ok]` marker.

---

## Overflow Policy: Reject-When-Full

| Condition | Action | Markers |
|-----------|--------|---------|
| `count < 16` | Push entry, increment count | `[bell.queue.push] id={} final_lane={} count={}` |
| `count >= 16` | Drop event, emit reject | `[bell.queue.reject.full] count=16` + `[bell.notify.reject] reason=queue_full` |

No silent overwrite. No dynamic growth.

---

## Handler Flow (Modified)

```
Parse fields
Validate enums ──→ fail → [bell.notify.reject] → continue
     │
Emit [bell.notify.recv]
     │
Derive lane (placeholder: no caps → PASSIVE)
Emit [bell.notify.downgrade] (if applicable)
     │
Push to BellQueue.push()
     ├── Ok(event_id) → [bell.queue.push] → [bell.notify.ok] → continue
     └── Err(full)    → [bell.queue.reject.full] → [bell.notify.reject] → continue
```

---

## Markers

| Marker | Budget | Fields | When |
|--------|--------|--------|------|
| `[bell.boot]` | — | — | sexbell starts |
| `[bell.notify.recv]` | 8 | caller_pd, category, requested | After validation |
| `[bell.notify.downgrade]` | 8 | from, to, reason | After lane derivation if downgraded |
| `[bell.queue.push]` | 64 | event_id, final_lane, count | After successful queue push |
| `[bell.notify.ok]` | 8 | caller_pd, final_lane, event_id | After queue push (updated to include event_id) |
| `[bell.queue.reject.full]` | 16 | count | When queue is full |
| `[bell.notify.reject]` | 4 | caller_pd, reason | On validation failure or queue full |
| `[bell.unknown.reject]` | 8 | type_id | Unmatched message type |

---

## Forbidden Features Confirmed Absent

| Feature | Check | Result |
|---------|-------|--------|
| Heap/alloc/Vec/String | `rg "Vec\|String\|alloc\|Box" sexbell/src/main.rs` | ❌ Absent |
| OP_BELL_LIST | `rg "OP_BELL_LIST" sexbell/src/main.rs` | ❌ Absent |
| Private content | `rg "title\|body\|sender_name" sexbell/src/main.rs` | ❌ Absent |
| Sender integration | `rg "pdx_call\|reply\|pdx_reply" sexbell/src/main.rs` | ❌ Absent (no reply path) |
| SilkBar/sexdisplay | `rg "silkbar\|sexdisplay\|0xEC\|0xEF" sexbell/src/main.rs` | ❌ Absent |
| Storage | `rg "store\|persist" sexbell/src/main.rs` | ❌ Absent |
| Action callbacks | `rg "action_callback\|action.*dispatch" sexbell/src/main.rs` | ❌ Absent |
| Sound | `rg "sound\|audio\|harp" sexbell/src/main.rs` | ❌ Absent |
| Kernel edits | `rg "MessageType::IpcCall.*OP_BELL_NOTIFY" kernel/src/init.rs` | ❌ No kernel enqueue |

---

## No-Kernel-Change Confirmation

The implementation is **sexbell-only**. No changes to:
- `kernel/src/init.rs` — no scaffold, no cap changes, no IpcCall enqueue
- `crates/sex-pdx/src/lib.rs` — no ABI, no opcode, no slot changes
- `limine.cfg` — no module list changes
- `sexos_build_spec.toml` — no build stage changes

---

## Build

```
[SEXOS ENTRYPOINT] success
```

---

## Next Phase

**BELL_RAM_QUEUE_PROOF_V1** — Add temporary kernel one-shot scaffold, boot QEMU, verify `[bell.queue.push]` marker appears. Then cleanup scaffold.

---

*End of BELL_RAM_QUEUE_IMPLEMENT_V1.md*
