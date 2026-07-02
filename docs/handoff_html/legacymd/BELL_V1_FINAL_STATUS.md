# Bell V1 Final Status

## Status: Complete (Phase A–D integrated, SilkBar presence merged)

---

## 1. Completed Bell V1 Features

### 1.1 Server Lifecycle
| Item | Status | Detail |
|------|--------|--------|
| Spawn | Done | PD 10, domain_id=10, deterministic order (last in module_paths) |
| Boot marker | `[bell.boot]` | Printed once at start of main loop |
| Listen loop | Done | `pdx_listen_raw(0)` — non-blocking, single-threaded |

### 1.2 Implemented Opcodes (6 of 8 defined in sex-pdx)

| Opcode | Constant | Handler | Status |
|--------|----------|---------|--------|
| NOTIFY | `OP_BELL_NOTIFY = 0xC0` | lines 376–535 | Done |
| CLOSE | `OP_BELL_CLOSE = 0xC1` | lines 702–741 | Done |
| ACTION | `OP_BELL_ACTION = 0xC2` | lines 744–789 | Done |
| LIST | `OP_BELL_LIST = 0xC3` | lines 542–699 | Done |
| CLEAR | `OP_BELL_CLEAR = 0xC4` | lines 792–846 | Done |
| MUTE_SENDER | `OP_BELL_MUTE_SENDER = 0xC7` | lines 848–922 | Done |
| SUBSCRIBE | `OP_BELL_SUBSCRIBE = 0xC5` | **Not implemented** → `[bell.unknown.reject]` | Phase E |
| SET_POLICY | `OP_BELL_SET_POLICY = 0xC6` | **Not implemented** → `[bell.unknown.reject]` | Phase E |

### 1.3 Queue
| Property | Value |
|----------|-------|
| Type | Fixed-size ring buffer |
| Capacity | 16 entries |
| Overflow | Drops lowest-priority entry (lane order: PASSIVE < INFO < WARN < NOTICE < ALERT < SECURITY) |
| Data per entry | event_id, caller_pd, category, requested_lane, final_lane, final_urgency, privacy_level, redaction_class, action_count, object_ref_count, dismissed flag |

### 1.4 Spam Budget
| Property | Value |
|----------|-------|
| Window | 64 ticks |
| Max per PD per window | 8 events |
| Tracking | 16-entry LRU-evicted slot array |
| Rejection marker | `[bell.notify.reject] reason=spam_budget_exceeded` |

### 1.5 Mute List
| Property | Value |
|----------|-------|
| Capacity | 16 PD slots |
| Structure | Static array with shift-remove |
| Operations | Add (slot available), Remove (by PD ID) |
| Rejection marker | `[bell.notify.reject] reason=muted` |

### 1.6 Privacy Model
| Level | Value | Behavior |
|-------|-------|----------|
| Public | 0 | Fully visible |
| Obfuscated | 1 | Exists but metadata hidden |
| StructuralMeta | 2 | Structural metadata may be hidden |
| FullHidden | 3 | Counted as redacted, no details revealed |

---

## 2. LIST Packed Reply Format

Callers receive reply as `msg.type_id == 1, msg.caller_pd == 1, msg.arg0 == packed_u64`.

```
Bit layout of packed reply (u64):
  [7:0]   = total_visible       (sum of all lane counts visible to caller)
  [15:8]  = lane0 count         (lanes 0=PASSIVE .. 5=SECURITY)
  [23:16] = lane1 count
  [31:24] = lane2 count
  [39:32] = lane3 count
  [47:40] = lane4 count
  [55:48] = lane5 count
  [63:56] = redacted_count      (FullHidden entries excluded from visible counts)

Error case (denied caller): bell_reply(caller_pd, u64::MAX)
```

---

## 3. SilkBar Presence Path

### 3.1 Poll Cadence
Every ~2 seconds (when `uptime_seconds % 2 == 0`).

### 3.2 LIST Call Arguments
- `lane_filter = 0xFF` (all lanes)
- `max_results = 1` (one item marker for debugging; Bell requires 1..=4)

### 3.3 Data Flow
```
SilkBar ──pdx_call(SLOT_BELL, OP_BELL_LIST)──→ Bell
                                                    │
                                              Bell validates args
                                              Bell checks allowlist (PD 6)
                                              Bell scans queue → aggregate counts
                                              Bell sends reply via syscall 29
                                                    │
                                                    ▼
SilkBar ←──pdx_try_listen_raw(0)──type_id=1, caller_pd=1, arg0=packed
    │
    └──send_update(SetBellPresence, packed)
          │
          ▼
    Sexdisplay applies to SilkBar.bell_state
    Sexdisplay renders Bell dot + count badge
```

### 3.4 Markers (after cleanup)
| Marker | Budget | When |
|--------|--------|------|
| `[bell.list.reply]` | 8 | Each successful LIST reply |
| `[silkbar.bell.poll.reply]` | 8 | SilkBar receives reply |
| `[sexdisplay.bell.render]` | 8 | Sexdisplay renders Bell dot |
| `[bell.list.redact]` | 8 | Privacy-redacted events exist |
| `[bell.list.item]` | 8 | Events exist and match filter |
| `[bell.list.reject]` (invalid_lane) | 4 | Invalid lane filter |
| `[bell.list.reject]` (invalid_count) | 4 | Invalid max_results |
| `[bell.readcap.deny]` | 8 | Unauthorized caller |
| `[silkbar.bell.reject]` | 8 | pdx_call failed |
| `[bell.list.reply.reject]` | — | Not implemented (hypothetical) |
| `[silkbar.bell.update]` | — | Not implemented (hypothetical) |

### 3.5 Render States (Bell dot color)
| Condition | Color | Meaning |
|-----------|-------|---------|
| `flags & 1 == 0` | Muted (dim) | Bell unavailable (no cap or server down) |
| `total_visible == 0` | Muted (dim) | No events |
| `redacted_count > 0` | Amber (0x00FFAA44) | Privacy-redacted events |
| Otherwise | Gold (0x00FFD700) | Active events |

Count badge (max 99) rendered in top-right of Bell module using FONT digits.

---

## 4. Current Grants and Allowlist

### 4.1 Kernel Capability Grants (init.rs)

| PD | Slot | Target | Why |
|----|------|--------|-----|
| silk-shell (3) | SLOT_BELL (12) | sexbell (10) | Shell policy control |
| silkbar (6) | SLOT_BELL (12) | sexbell (10) | Poll for aggregate presence |
| sexbell (10) | SLOT_BELL (12) | sexbell (10) | Self-cap for listen loop |

### 4.2 Server-Side Allowlist (sexbell/src/main.rs)

```
BELL_LIST_ALLOWLIST: &[u32] = &[3, 6]
// 3 = silk-shell (policy owner)
// 6 = silkbar (privacy-safe aggregate poller)
```

Default-deny: any PD not in this list is rejected with `u64::MAX` reply.

### 4.3 Two-Gate Model
1. **Kernel gate**: Capability grant (SLOT_BELL domain capability)
2. **Server gate**: Allowlist check (BELL_LIST_ALLOWLIST)

Both must pass for LIST to succeed.

---

## 5. Privacy Guarantees

| Guarantee | Implementation |
|-----------|---------------|
| No sender identity leak | Aggregate lane counts only — no caller_pd in reply |
| No content leak | Queue scan filters by `caller_max_privacy` derived from allowlist |
| FullHidden non-existence deniability | Redacted events counted but never revealed |
| No action callbacks exposed | ACTION opcode requires explicit event_id; not exposed via LIST |
| No queue enumeration | max_results limited to 1..=4, accept no wildcard/negative values |
| Default-deny LIST | Only 2 PDs allowlisted |

---

## 6. Known `sex_pdx::pdx_reply` Syscall Mismatch

`sex_pdx::pdx_reply()` calls syscall 1, but the kernel implements `SYSCALL_PDX_REPLY` at syscall 29.

**Impact**: Any server using `sex_pdx::pdx_reply()` will silently fail to send replies.

**Workaround in Bell**: `bell_reply()` uses inline asm with syscall 29 directly.

**Other servers affected**: sexstore uses syscall 29 directly (inline asm).

**Fix**: Unify sex-pdx to use syscall 29. Out of scope for Bell V1.

---

## 7. Blocked Phase E Items

The following sex-pdx opcodes are defined but **not implemented** in sexbell.
Any call to these opcodes falls through to `[bell.unknown.reject]`.

| Opcode | Constant | Purpose | Blocked By |
|--------|----------|---------|-----------|
| SUBSCRIBE | `OP_BELL_SUBSCRIBE = 0xC5` | SilkBar subscribes to lane-summary push updates | Requires kernel: no push IPC, no shared-memory. Would eliminate polling. |
| SET_POLICY | `OP_BELL_SET_POLICY = 0xC6` | Per-app user policy overrides (privacy level, mute) | Requires no new dependencies, but no existing mechanism for persistent per-app policy storage. |

### Why Phase E is Cleanly Blocked
- **SUBSCRIBE** needs a kernel push mechanism (IPC callback or shared ring buffer).
  Current IPC model is async-enqueue + poll. No push notification channel exists.
  Would require kernel ABI changes (new syscall or shared-memory region).
- **SET_POLICY** needs persistent storage and per-app policy objects.
  Sexstore exists but has no schema for Bell policy. No policy-engine exists.

---

## 8. Recommended Next Phases

### Phase E (Blocked — requires kernel ABI changes)
1. **SUBSCRIBE**: Add kernel push IPC or shared-memory notification ring.
2. **SET_POLICY**: Design per-app policy schema in sexstore; add policy engine to Bell.

### Post-V1 Housekeeping
1. **Fix sex_pdx::pdx_reply syscall number** (syscall 1 → syscall 29).
   Single change in `crates/sex-pdx/src/lib.rs`. No behavioral change.
2. **Remove dead comments** in SilkBar about V1 lacking SLOT_BELL grant
   (grant is now present since commit `95f6e96`).

### Future (No timeline)
- Notification center UI (SilkDrop surface)
- Per-app notification preferences panel
- Notification history persistence (sexstore-backed)
- Multi-lane subscribe granularity

---

## 9. Files and Commits

### Core Bell Server
- `servers/sexbell/src/main.rs` (930 lines)

### Integration
- `servers/silkbar/src/main.rs` — Bell poll every 2s, reply listener
- `servers/sexdisplay/src/main.rs` — Bell dot + count badge rendering
- `kernel/src/init.rs` — SLOT_BELL capability grants (PDs 3, 6, 10)
- `crates/silkbar-model/src/lib.rs` — `BellState` struct, `SetBellPresence = 7`

### Protocol
- `crates/sex-pdx/src/lib.rs` — 8 opcodes (0xC0–0xC7), `SLOT_BELL = 12`

### Key Commits (chronological)
```
8ade85e feat(bell): implement spam budget and queue overflow dropping
66d4090 feat(bell): accept action callbacks and object references, implement OP_BELL_ACTION
1a15173 feat(bell): add action_id and object_ref fields
218f5b9 feat(bell): implement privacy enforcement in OP_BELL_LIST
d70e2b1 fix(bell): add LIST reply with aggregate lane counts
95f6e96 feat(bell): complete V1 presence pipeline
4ba4277 feat(silkbar): add Bell V1 aggregate presence display
8a7bf5b fix(bell): complete SilkBar presence reply route
0e769cf chore(bell): clean presence proof marker noise
```

---

## 10. Handoff Documents Index

All Bell V1 handoff documents are in `docs/handoff/BELL_*.md` (42 files).
The sequence follows: Plan → Implement → Proof → Cleanup → Freeze.

Key documents for future phases:
- `BELL_EVENT_MODEL_DESIGN_GATE_V1.md` — Original design document
- `BELL_PDX_PROTOCOL_SPEC_V1.md` — Wire format and opcode semantics
- `BELL_CAPABILITY_POLICY_V1.md` — Two-gate model and allowlist design
- `BELL_PHASE1_FREEZE_V1.md` — Bell V1 phase boundary
- `BELL_V1_FINAL_STATUS.md` — This document
