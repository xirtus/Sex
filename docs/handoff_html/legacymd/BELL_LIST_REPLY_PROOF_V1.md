# Bell LIST Reply Proof

**Status:** Implemented.
**Date:** 2026-05-06
**Files changed:** `servers/sexbell/src/main.rs` (+66/-1 lines)
**Build:** `./scripts/entrypoint_build.sh` — PASS

---

## 1. Current Reply Mechanism Found

The kernel's IPC reply mechanism uses **syscall 29** (`SYSCALL_PDX_REPLY`), not syscall 1 as defined in `sex_pdx::pdx_reply()`:

| Component | Syscall number | Status |
|---|---|---|
| `sex_pdx::pdx_reply()` | 1 | ❌ Unhandled by kernel (falls to `u64::MAX`) |
| Kernel `SYSCALL_PDX_REPLY` | 29 | ✅ Active handler in `kernel/src/syscalls/mod.rs` |
| Other servers' workaround | 29 | ✅ `sexstore`, `sexdrive`, etc. use syscall 29 directly via inline asm |

This mismatch is documented in `sexstore`:
> "sex-pdx's pdx_reply() uses syscall 1 — unhandled in current kernel. Use 29 directly."

**The fix uses the same pattern** as `sexstore` — an inline `bell_reply()` function calling syscall 29 with `rdi=target_pd`, `rsi=val`.

### Reply flow

```
Caller                          Bell (target)
  │                                │
  ├─ pdx_call(SLOT_BELL,           │
  │    OP_BELL_LIST, args) ──────► │  (syscall 0 → AsyncEnqueue to msg ring)
  │  returns (0, 0) immediately   │
  │                                ├─ pdx_listen_raw(0)
  │                                │    → receives IpcCall(func_id=0xC3, ...)
  │                                ├─ compute lane counts
  │                                ├─ bell_reply(caller_pd, packed) ──►  (syscall 29)
  │                                │    → pushed to caller's incoming_replies
  │  ┌─────────────────────────────┘
  │  ▼
  ├─ pdx_listen_raw(0)  (later loop iteration)
  │    → receives (type_id=1, arg0=packed_counts)
  │    → processes reply
```

The caller sends the request via `pdx_call` (returns immediately — async enqueue) and later picks up the reply via `pdx_listen_raw(0)` where `type_id=1` with `caller_pd=1` signals a pending reply, and `arg0` contains the packed value.

---

## 2. Packed Reply Format

A single `u64` returned in `arg0` of the reply message:

```
Bit layout (8 bytes):
  63      56  55      48  47      40  39      32  31      24  23      16  15       8  7        0
┌──────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────┬──────────┐
│ redacted │  lane 5  │  lane 4  │  lane 3  │  lane 2  │  lane 1  │  lane 0  │   total  │
│  count   │ SECURITY │ PROJECT  │  SYSTEM  │  LATER   │   SOON   │ NOW/PASS │  visible │
└──────────┴──────────┴──────────┴──────────┴──────────┴──────────┴──────────┴──────────┘
```

| Field | Bits | Max value | Notes |
|---|---|---|---|
| `total_visible` | 7:0 | 255 (actual queue max: 16) | Sum of all visible (non-dismissed, non-redacted) events |
| `lane_counts[0]` (NOW/PASSIVE) | 15:8 | 255 | Events in lane 0 |
| `lane_counts[1]` (SOON) | 23:16 | 255 | Events in lane 1 |
| `lane_counts[2]` (LATER) | 31:24 | 255 | Events in lane 2 |
| `lane_counts[3]` (SYSTEM) | 39:32 | 255 | Events in lane 3 |
| `lane_counts[4]` (PROJECT) | 47:40 | 255 | Events in lane 4 |
| `lane_counts[5]` (SECURITY) | 55:48 | 255 | Events in lane 5 |
| `redact_count` | 63:56 | 255 | FullHidden events skipped by privacy gate |

**Error responses:**

| Condition | Reply value |
|---|---|
| Caller not in LIST allowlist | `u64::MAX` (all bits 1) |
| Success | Packed counts as above |

**Future extensibility:** Bits 63-56 currently hold `redact_count`. If flags are needed in the future (overflow, mute indicator), `redact_count` can move or share space — max queue is 16 entries so 8 bits for counts is generous.

---

## 3. Privacy Result

| Concern | Status |
|---|---|
| Reply contains only aggregate counts | ✅ No event IDs, no sender PDs, no body/content |
| FullHidden events excluded from visible counts | ✅ Privacy gate skips entries above caller's max privacy level |
| FullHidden count revealed only as `redact_count` | ✅ Redacted count is aggregate, not per-event |
| No caller PD leakage | ✅ `caller_pd` is kernel-authoritative, not user-supplied |
| No sender identity in reply | ✅ Aggregated by lane only |
| Budgeted markers only | ✅ `[bell.list.reply]` marker has budget of 8 |

---

## 4. Marker Changes

| Marker | Budget | Source | Status |
|---|---|---|---|
| `[bell.list.reply] total=.. lanes=[..] redacted=..` | 8 | New | ✅ Budgeted aggregate-only marker |
| `[bell.readcap.deny]` (refined) | 8 | Existing (now also calls `bell_reply`) | ✅ Also replies with error |

No markers removed. All marker budgets are independent static `u32` counters.

---

## 5. Changed Files

| File | Lines changed | Change |
|---|---|---|
| `servers/sexbell/src/main.rs` | +66/-1 | Added `bell_reply()` function, aggregate lane counting, packed reply in LIST handler, error reply for denied callers |

No kernel, sex-pdx, SilkBar, sexdisplay, or model changes.

---

## 6. Build Result

**PASS** — `sexos-v1.0.0.iso` produced successfully.

---

## 7. SilkBar Polling Readiness

Now that Bell replies, SilkBar polling can be implemented **without any further Bell-side changes**:

| Requirement | Status |
|---|---|
| Bell replies to LIST with aggregate counts | ✅ Done |
| Reply format defined | ✅ 8-byte packed layout |
| Error case handled (not in allowlist) | ✅ Returns `u64::MAX` |
| No kernel/ABI changes needed | ✅ Confirmed |
| Privacy-safe | ✅ Aggregate only |

### What SilkBar would need on its side (future, contract-gated):

1. **silkbar-model**: Add `UpdateKind::SetBellPresence = 7` (STOP FIRST — model ABI change)
2. **silkbar**: Call `pdx_call(SLOT_BELL, OP_BELL_LIST, 0xFF, 0, 0)` every ~2s, pick up reply via `pdx_try_listen_raw(0)` checking `type_id=1`
3. **sexdisplay**: Handle `SetBellPresence` and render dot + count

All three are **blocked on contract review**, not on Bell capability.
