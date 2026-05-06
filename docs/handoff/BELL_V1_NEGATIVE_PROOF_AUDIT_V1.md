# BELL V1 Negative Proof Audit

**Audit date:** 2026-05-06
**Scope:** Bell Phases A–D implementation in `servers/sexbell/src/main.rs` (905 lines)
**ABI reference:** `crates/sex-pdx/src/lib.rs`
**Build:** `./scripts/entrypoint_build.sh` — PASS

---

## 1. A–D Source Verification Table

| Phase | Claim | Verified? | Source lines | Notes |
|---|---|---|---|---|
| **A** | `OP_BELL_NOTIFY` (0xC0) creates event in queue | ✅ PASS | 358–521 | Parses fields, validates, derives lane, checks spam, pushes to queue |
| **A** | `OP_BELL_CLOSE` (0xC1) dismisses event by ID | ✅ PASS | 677–716 | Searches queue head→tail for event_id, sets dismissed=1 |
| **A** | `OP_BELL_ACTION` (0xC2) dispatches marker only | ✅ PASS | 719–765 | Searches queue for event_id + action_id match, emits marker, **no callback execution** |
| **B** | `OP_BELL_LIST` (0xC3) lists event summaries | ✅ PASS | 524–674 | Newest-first iteration, skip dismissed, privacy gate, max_results cap |
| **B** | `OP_BELL_CLEAR` (0xC4) clears lane or all lanes | ✅ PASS | 767–820 | lane_filter=0xFF resets queue; lane≤5 marks matching dismissed=1 |
| **C** | `OP_BELL_MUTE_SENDER` (0xC7) mutes/unmutes PDs | ✅ PASS | 823–890 | Static `MUTE_LIST[16]`, action=0 add, action=1 remove, idempotent add, shift-remove |
| **C** | Spam budget: 8 events / 62 ticks / 16 PD slots | ✅ PASS | 253–308 | `check_spam_budget()`: window=62 ticks, max=8 per window, slots=16 with LRU eviction |
| **D** | Queue overflow drops lowest-priority entry | ✅ PASS | 124–150 | `find_lowest_priority_index()` picks lowest `final_lane`, ties broken by oldest; writes new entry into freed slot |
| **D** | action_count/object_ref_count accept max 1 | ✅ PASS | 395–400 | Validates `action_count > 1` and `action_count==1 && action_id==0` reject; same for `object_ref_count > 1` |
| **D** | FullHidden privacy redaction in LIST | ✅ PASS | 612–618, 642–653 | Entries with privacy_level > caller_max_privacy are skipped; FullHidden (level 3) increments `redact_count`; `[bell.list.redact]` marker emitted |

---

## 2. Negative Proof Markers

Every negative path emits a **budgeted, metadata-only** marker. No event body/title/private content leaks.

| Negative path | Marker format | Budget | Source lines | Status |
|---|---|---|---|---|
| **Muted sender** | `[bell.notify.reject] caller_pd=.. reason=muted` | 8 | 371–381 | ✅ |
| **Invalid category** | `[bell.notify.reject] caller_pd=.. reason=invalid_category` | 4 | 403–413 | ✅ |
| **Invalid privacy** | `[bell.notify.reject] caller_pd=.. reason=invalid_privacy` | 4 | 403–413 | ✅ |
| **Invalid redaction** | `[bell.notify.reject] caller_pd=.. reason=invalid_redaction` | 4 | 403–413 | ✅ |
| **Invalid urgency** | `[bell.notify.reject] caller_pd=.. reason=invalid_urgency` | 4 | 403–413 | ✅ |
| **action_count > 1** | `[bell.notify.reject] caller_pd=.. reason=action_count_invalid` | 4 | 403–413 | ✅ |
| **action_count==1, action_id==0** | `[bell.notify.reject] caller_pd=.. reason=action_id_zero` | 4 | 403–413 | ✅ |
| **object_ref_count > 1** | `[bell.notify.reject] caller_pd=.. reason=object_refs_invalid` | 4 | 403–413 | ✅ |
| **Spam budget exceeded** | `[bell.notify.reject] caller_pd=.. reason=spam_budget_exceeded window=64 max=8` | 8 | 444–454 | ✅ |
| **Queue overflow (drop)** | `[bell.queue.drop] reason=lowest_priority lane=.. dropped_lane=.. event_id=..` | 16 | 468–477 | ✅ |
| **Queue full (no drop)** | `[bell.queue.reject.full] count=16` → `[bell.notify.reject] caller_pd=.. reason=queue_full` | 16+4 | 500–519 | ✅ |
| **Close not found** | `[bell.close.reject] reason=not_found event_id=.. caller_pd=..` | 4 | 706–715 | ✅ |
| **Action not found** | `[bell.action.reject] reason=not_found event_id=.. action_id=.. caller_pd=..` | 4 | 754–763 | ✅ |
| **Clear invalid lane** | `[bell.clear.reject] reason=invalid_lane lane=.. caller_pd=..` | 4 | 810–819 | ✅ |
| **Mute list full** | `[bell.mute.reject] reason=mute_list_full mute_pd=.. caller_pd=..` | 4 | 843–852 | ✅ |
| **Mute remove not found** | `[bell.mute.reject] reason=not_found mute_pd=.. caller_pd=..` | 4 | 867–876 | ✅ |
| **Mute invalid action** | `[bell.mute.reject] reason=invalid_action action=.. caller_pd=..` | 4 | 879–889 | ✅ |
| **List invalid lane** | `[bell.list.reject] reason=invalid_lane caller_pd=..` | 4 | 532–542 | ✅ |
| **List invalid count** | `[bell.list.reject] reason=invalid_count caller_pd=..` | 4 | 546–556 | ✅ |
| **List not allowed (read-cap deny)** | `[bell.readcap.deny] caller_pd=.. op=list reason=no_read_cap` | 8 | 563–573 | ✅ |
| **Unsupported opcode** | `[bell.unknown.reject] type_id=0x..` | 8 | 893–901 | ✅ |
| **FullHidden redaction** | `[bell.list.redact] reason=full_hidden count=.. caller_pd=..` | 8 | 643–652 | ✅ |

**Total budgets consumed per boot:** at most ~190 serial writes across all negative paths.

---

## 3. Privacy / Log Redaction

| Concern | Status | Evidence |
|---|---|---|
| Event body/title never logged | ✅ | All markers log only metadata: `caller_pd`, `category`, `lane`, `event_id`, counts |
| FullHidden content not revealed | ✅ | `privacy_level==3` entries skip LIST output entirely; only `redact_count` is logged |
| `caller_pd` is safe (kernel-authoritative) | ✅ | Only kernel-provided `msg.caller_pd` is used, not user-supplied values |
| No private data in reject reasons | ✅ | Reject reasons are static `&'static str` constants, no dynamic content |
| No private data in mute/unmute | ✅ | Only PD numbers and budgeted markers |
| Budget exhaustion stops all logs | ✅ | Every `static mut .._BUDGET` decrements to 0, then silences |

---

## 4. Bounded Memory Verification

| Structure | Size | Bound | Notes |
|---|---|---|---|
| `BellQueue.entries[]` | 16 × `BellQueueEntry` (56 bytes each) = **896 bytes** | `BELL_QUEUE_CAPACITY=16` | Fixed-size ring buffer, no dynamic allocation |
| `MUTE_LIST[]` | 16 × `u32` = **64 bytes** | `MUTE_LIST_CAPACITY=16` | Static array, shift-remove |
| `SPAM_BUDGET.slots[]` | 16 × `(u32,u32,u64)` = **192 bytes** | `SPAM_BUDGET_SLOTS=16` | Static array, LRU eviction |
| All budget counters | ~25 × `u32` = **100 bytes** | Each decrements to 0 | Static `mut` |
| Total Bell RAM | **~1.3 KB** | All fixed-size | No `Vec`, no `Box`, no heap |

---

## 5. STOP FIRST Issues

| Issue | Status |
|---|---|
| ABI/opcode changes required? | **None.** All opcodes are already defined in sex-pdx. |
| Collar PD/cap system changes? | **None.** No Collar dependency found in sexbell (grep confirmed). |
| Storage/audio/shell lifecycle changes? | **None.** OP_BELL_SUBSCRIBE (0xC5) and OP_BELL_SET_POLICY (0xC6) are defined but **not implemented** — fall through to `[bell.unknown.reject]`. Correct for V1. |
| Privacy leak in current logs? | **None.** All markers are metadata-only. |
| Kernel/sexdisplay changes needed? | **None.** |

---

## 6. Silk-Shell Local Bell Ring (out of scope, noted)

`servers/silk-shell/src/main.rs` has a shell-local `BELL_EVENTS[16]` ring buffer for `BellEventKind::ObjectLinkedToBuffer` stubs (J7). This is **independent** from sexbell:

- Does not call any `OP_BELL_*` opcode
- Does not import or reference sexbell
- No shared state with sexbell
- Used only for Linen→Quil object-link event rendering in the Atlas scene

No action needed.

---

## 7. Changed Files

| File | Change | Status |
|---|---|---|
| `servers/sexbell/src/main.rs` | Pre-existing Bell implementation (no changes needed by audit) | ✅ Unchanged |
| `docs/handoff/BELL_V1_NEGATIVE_PROOF_AUDIT_V1.md` | This audit document | ✅ New |

---

## 8. Remaining Blocked Work (Phase E+)

| Feature | Blocked by | Notes |
|---|---|---|
| Collar integration | Phase E | Not started. Requires Collar PD/cap system. |
| OP_BELL_SUBSCRIBE (0xC5) | SilkBar server | Not implemented. Current Bell rejects with unknown. |
| OP_BELL_SET_POLICY (0xC6) | Policy table, UI | Not implemented. Current Bell rejects with unknown. |
| Action callback execution | Collar dispatch | V1 is marker-only. No real dispatch. |
| Object reference resolution | Collar namespace | V1 stores refs, does not resolve them. |
| Lane derivation with caps | BellCap table | First-proof placeholder: all → PASSIVE(0). |

---

## 9. Verification Summary

| Invariant | Result |
|---|---|
| All A-D claims verified from source | ✅ PASS |
| All negative paths emit budgeted markers | ✅ PASS (22 negative paths) |
| No private content in logs | ✅ PASS |
| All queues/tables fixed-size bounded | ✅ PASS (~1.3 KB total) |
| OP_BELL_ACTION is marker-only | ✅ PASS |
| No Collar/storage/audio dependency | ✅ PASS |
| No kernel/ABI/sexdisplay changes | ✅ PASS |
| Build passes | ✅ PASS |
