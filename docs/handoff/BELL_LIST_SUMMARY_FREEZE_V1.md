# BELL_LIST_SUMMARY_FREEZE_V1

**Status:** Bell Phase 3 list summary API complete. Frozen.
**Build:** `[SEXOS ENTRYPOINT] success`
**Date:** 2026-05-05
**Depends on:** `BELL_LIST_SUMMARY_PLAN_V1.md`, `BELL_LIST_SUMMARY_IMPLEMENT_V1.md`, `BELL_LIST_SUMMARY_POPULATED_PROOF_V1.md`, `BELL_LIST_SUMMARY_CLEANUP_V1.md`, `BELL_RAM_QUEUE_FREEZE_V1.md`

---

## 1. Final OP_BELL_LIST State

### Opcode assignment

| Constant | Value | Source |
|----------|-------|--------|
| `OP_BELL_LIST` | 0xC3 | `crates/sex-pdx/src/lib.rs:109` |
| `SLOT_BELL` | 12 | `crates/sex-pdx/src/lib.rs:368` |

### Handler location

`servers/sexbell/src/main.rs`, lines 276-371, in the main dispatch `match msg.type_id { ... }` block.

### Request shape

| Arg | Bits | Field | Values |
|-----|------|-------|--------|
| `arg0` | 0-7 | `lane_filter` | 0xFF (all) or 0..=5 (specific lane) |
| `arg0` | 8-15 | `max_results` | 1..=4 |
| `arg0` | 16-63 | `_reserved` | 0 (ignored) |

### Reply strategy

**Marker-only V1.** No reply ABI. No `pdx_reply` call. No `pdx_call` return value. No shared memory ring.

### Dispatch path

```
OP_BELL_LIST (0xC3)
  ├── Parse lane_filter, max_results from arg0
  ├── Validate lane_filter (0xFF or 0..=5)
  │     └── Invalid → [bell.list.reject] reason=invalid_lane → continue
  ├── Validate max_results (1..=4)
  │     └── Invalid → [bell.list.reject] reason=invalid_count → continue
  ├── [bell.list.recv] lane_filter max_results caller_pd
  ├── Iterate queue newest-first, apply lane_filter against final_lane
  │     ├── Match → [bell.list.item] event_id final_lane category privacy redaction
  │     │              match_count++
  │     │              if match_count >= max_results: break
  │     └── No match → continue
  ├── If match_count == 0 → [bell.list.empty]
  └── If match_count > 0  → [bell.list.done] count=match_count
```

---

## 2. Validation Rules (Final)

| Rule | Check | Reject Reason |
|------|-------|---------------|
| lane_filter valid | `lane_filter == 0xFF` or `lane_filter <= 5` | `invalid_lane` |
| max_results valid | `1 <= max_results <= 4` | `invalid_count` |
| max_results == 0 | Reject | `invalid_count` |
| max_results > 4 | Reject (no clamping) | `invalid_count` |
| lane_filter > 5 (and not 0xFF) | Reject | `invalid_lane` |

---

## 3. Queue Read Behavior (Final)

### Newest-first iteration

```rust
for i in 0..BELL_QUEUE.count as usize {
    let idx = (BELL_QUEUE.tail as usize + BELL_QUEUE_CAPACITY - 1 - i) % BELL_QUEUE_CAPACITY;
    let entry = &BELL_QUEUE.entries[idx];
    // apply lane_filter, emit marker, count
}
```

- Index wraps around the ring buffer (newest = tail-1, oldest = head)
- Lane filter: `lane_filter == 0xFF` matches all; else `entry.final_lane == lane_filter`
- Stops early when `match_count >= max_results`
- **Queue is NOT mutated** — read-only traversal

### Summary fields in `[bell.list.item]`

| Field | Source | Classification |
|-------|--------|----------------|
| `event_id` | `entry.event_id` (u64) | StructuralMeta |
| `final_lane` | `entry.final_lane` (u8) | StructuralMeta |
| `category` | `entry.category` (u8) | StructuralMeta |
| `privacy` | `entry.privacy_level` (u8) | StructuralMeta |
| `redaction` | `entry.redaction_class` (u8) | StructuralMeta |

All five fields are read directly from the stored `BellQueueEntry` with no transformation. No private content (title, body, sender name, file paths, object names) is exposed.

---

## 4. Markers (Final)

| Marker | Budget | Fields | When |
|--------|--------|--------|------|
| `[bell.list.recv]` | 8 | `lane_filter`, `max_results`, `caller_pd` | Valid request received |
| `[bell.list.item]` | 16 | `event_id`, `final_lane`, `category`, `privacy`, `redaction` | Per matching entry |
| `[bell.list.empty]` | 4 | — | 0 matching entries |
| `[bell.list.done]` | 8 | `count` | Non-empty result after items |
| `[bell.list.reject]` | 4 | `reason`, `caller_pd` | Invalid request |

All budgets are `static mut` counters with rate-limiting (decrement on emit, stop at 0).

---

## 5. Proof History

| Proof | Queue State | Markers Observed | Status |
|-------|-------------|-------------------|--------|
| Empty-queue proof | 0 entries | `[bell.list.recv]` + `[bell.list.empty]` | ✅ Passed |
| Populated-queue proof (qemuX.sh) | 1 entry (event_id=1) | `[bell.list.recv]` + `[bell.list.item] event_id=1 final_lane=0 category=0 privacy=0 redaction=0` + `[bell.list.done] count=1` | ✅ Passed |

Both proofs used a temporary kernel scaffold (notify+list for populated, list-only for empty). All scaffolds removed in cleanup.

### Runtime proof convention

All future QEMU proofs use `./qemuX.sh` — patched QEMU with XHCI/HID fixes, `-M q35,i8042=off`, USB-only input, `-display sdl`.

---

## 6. Scaffold Absence Confirmation

```bash
$ rg -n "kernel\.sexbell\." kernel/src/init.rs
175: serial_println!("[kernel.sexbell.cap] self slot=12")
# Only permanent self-cap grant. No test/populate/list enqueues.
```

All temporary kernel test enqueues from Bell Phase 3 are removed:

| Scaffold | Removed? |
|----------|----------|
| `[kernel.sexbell.list.test]` (OP_BELL_LIST enqueue) | ✅ |
| `[kernel.sexbell.list.populate.test]` (OP_BELL_NOTIFY + OP_BELL_LIST) | ✅ |
| Any other `MessageType::IpcCall` with `OP_BELL_LIST` | ✅ |

---

## 7. Forbidden Features — Confirmed Absent

| Feature | Check | Result |
|---------|-------|--------|
| Reply ABI (`pdx_reply`/`pdx_call`) | `rg` on sexbell/main.rs | ❌ Absent |
| Heap/alloc/Vec/String/Box | `rg` on sexbell/main.rs | ❌ Absent |
| SilkBar integration | `rg` on sexbell/main.rs | ❌ Absent |
| Storage/persistence | `rg` on sexbell/main.rs | ❌ Absent |
| Private content (title/body/sender/file) | `rg` on sexbell/main.rs | ❌ Absent |
| Queue mutation (`OP_BELL_CLEAR`) | `rg` on sexbell/main.rs | ❌ Absent |
| External reader caps | `rg "SLOT_BELL.*Domain" init.rs` | ❌ Only self-cap |
| Action callbacks | `rg` on sexbell/main.rs | ❌ Absent |
| Sound/audio | `rg` on sexbell/main.rs | ❌ Absent |
| sex-pdx edits | No changes since Phase 1 | ❌ Unchanged |
| Kernel ABI changes | No new syscalls | ❌ Unchanged |

---

## 8. sex-pdx Constants (Unchanged Since Phase 1)

| Constant | Value | Status |
|----------|-------|--------|
| `SLOT_BELL` | 12 | ✅ Final (Phase 1) |
| `OP_BELL_NOTIFY` | 0xC0 | ✅ Final (Phase 1) |
| `OP_BELL_CLOSE` | 0xC1 | ✅ Reserved, unused |
| `OP_BELL_ACTION` | 0xC2 | ✅ Reserved, unused |
| `OP_BELL_LIST` | 0xC3 | ✅ **Active (Phase 3)** |
| `OP_BELL_CLEAR` | 0xC4 | ✅ Reserved, unused |
| `OP_BELL_SUBSCRIBE` | 0xC5 | ✅ Reserved, unused |
| `OP_BELL_SET_POLICY` | 0xC6 | ✅ Reserved, unused |
| `OP_BELL_MUTE_SENDER` | 0xC7 | ✅ Reserved, unused |

---

## 9. Known Limitations

| Limitation | Impact | Future Phase |
|------------|--------|--------------|
| Marker-only; no structured reply ABI | No real caller can read summaries | Reply ABI design |
| No real reader/sender caps | Only kernel scaffold can call OP_BELL_LIST | `BELL_READER_CAP_PLAN_V1` |
| No SilkBar presence | No visual indicator of event count | `BELL_SILKBAR_PRESENCE_PLAN_V1` |
| No inbox UI | Events not displayed to user | After SilkBar presence |
| No persistence/history beyond RAM queue | Events lost on reboot | E-series storage gate |
| No clear/mute/action behavior | Queue can only grow to 16 then reject | `OP_BELL_CLEAR` phase |
| No private content transport | Title/body not on wire | Content-token design gate |
| No queue-full list proof | List behavior at queue capacity untested | Deferred overflow proof |
| No sound/audio integration | No urgency-based audio hints | Harp/Theremin gate |

---

## 10. Phase 3 Verdict

**Bell Phase 3 (list summary) is frozen.**

| Component | Status |
|-----------|--------|
| OP_BELL_LIST handler | ✅ Complete (marker-only, no reply, no caps) |
| Request validation | ✅ Complete (lane_filter, max_results 1..=4) |
| Queue read traversal | ✅ Complete (newest-first, no mutation) |
| Empty-queue proof | ✅ Complete |
| Populated-queue proof | ✅ Complete (event_id=1, item+done markers) |
| Scaffolds removed | ✅ All cleaned |
| sex-pdx edits | ✅ None needed (OP_BELL_LIST=0xC3 already assigned) |
| Kernel edits | ✅ None beyond temporary scaffold (removed) |
| Runtime proof convention | ✅ `qemuX.sh` documented |

---

## 11. Complete Bell State (All Phases)

| # | Phase | Handoff | Status |
|---|-------|---------|--------|
| 1 | Event model design | `BELL_EVENT_MODEL_DESIGN_GATE_V1.md` | ✅ Done |
| 2 | Cap policy | `BELL_CAPABILITY_POLICY_V1.md` | ✅ Done |
| 3 | Protocol spec | `BELL_PDX_PROTOCOL_SPEC_V1.md` | ✅ Done |
| 4 | Namespace audit | `BELL_NAMESPACE_COLLISION_AUDIT_V1.md` | ✅ Done |
| 5 | Slot/opcode assignment | `BELL_SLOT_OPCODE_ASSIGNMENT_V1.md` | ✅ Done |
| 6 | Server stub plan | `BELL_SERVER_STUB_PLAN_V1.md` | ✅ Done |
| 7 | Server stub | `BELL_SERVER_STUB_V1.md` | ✅ Done |
| 8 | Boot spawn plan | `BELL_BOOT_SPAWN_PLAN_V1.md` | ✅ Done |
| 9 | Boot spawn | `BELL_BOOT_SPAWN_V1.md` | ✅ Done |
| 10 | Spawn proof | `BELL_SPAWN_PROOF_V1.md` | ✅ Done |
| 11 | Unknown reject proof | `BELL_UNKNOWN_REJECT_PROOF_V1.md` | ✅ Done |
| 12 | Unknown reject cleanup | `BELL_UNKNOWN_REJECT_CLEANUP_V1.md` | ✅ Done |
| 13 | Notify plan | `BELL_NOTIFY_PLAN_V1.md` | ✅ Done |
| 14 | Notify implement | `BELL_NOTIFY_IMPLEMENT_V1.md` | ✅ Done |
| 15 | Notify proof | `BELL_NOTIFY_PROOF_V1.md` | ✅ Done |
| 16 | Notify cleanup | `BELL_NOTIFY_CLEANUP_V1.md` | ✅ Done |
| 17 | Negative plan | `BELL_NOTIFY_NEGATIVE_PROOF_PLAN_V1.md` | ✅ Done |
| 18 | Negative proof | `BELL_NOTIFY_NEGATIVE_PROOF_V1.md` | ✅ Done |
| 19 | Negative cleanup | `BELL_NOTIFY_NEGATIVE_CLEANUP_V1.md` | ✅ Done |
| 20 | Phase 1 freeze | `BELL_PHASE1_FREEZE_V1.md` | ✅ Frozen |
| 21 | RAM queue plan | `BELL_RAM_QUEUE_PLAN_V1.md` | ✅ Done |
| 22 | RAM queue implement | `BELL_RAM_QUEUE_IMPLEMENT_V1.md` | ✅ Done |
| 23 | RAM queue proof | `BELL_RAM_QUEUE_PROOF_V1.md` | ✅ Done |
| 24 | RAM queue cleanup | `BELL_RAM_QUEUE_CLEANUP_V1.md` | ✅ Done |
| 25 | RAM queue freeze | `BELL_RAM_QUEUE_FREEZE_V1.md` | ✅ Frozen |
| **26** | **List summary plan** | **`BELL_LIST_SUMMARY_PLAN_V1.md`** | **✅ Done** |
| **27** | **List summary implement** | **`BELL_LIST_SUMMARY_IMPLEMENT_V1.md`** | **✅ Done** |
| **28** | **List summary proof** | **`BELL_LIST_SUMMARY_POPULATED_PROOF_V1.md`** | **✅ Done** |
| **29** | **List summary cleanup** | **`BELL_LIST_SUMMARY_CLEANUP_V1.md`** | **✅ Done** |
| **30** | **List summary freeze** | **`BELL_LIST_SUMMARY_FREEZE_V1.md`** | **✅ Here** |

**28 completed phases, 2 frozen checkpoints.**

---

## 12. Next Recommended Phase

**BELL_READER_CAP_PLAN_V1** — Design a read-cap policy so real senders (silk-shell, SilkBar) can call `OP_BELL_LIST`. This requires:
- Read-capability table in sexbell (which domains may call OP_BELL_LIST)
- `SLOT_BELL` cap grant to approved domains in kernel init.rs
- Marker-only or reply-path decision for real senders
- Default-deny for all unlisted domains

After reader caps: SilkBar presence design → inbox UI.

---

## References

- All 28 prior Bell handoff documents in `docs/handoff/BELL_*.md`
- `servers/sexbell/src/main.rs` — OP_BELL_LIST handler + RAM queue
- `kernel/src/init.rs` — spawn + self-cap (no enqueues)
- `crates/sex-pdx/src/lib.rs` — OP_BELL_LIST=0xC3 constant

---

*End of BELL_LIST_SUMMARY_FREEZE_V1.md*
