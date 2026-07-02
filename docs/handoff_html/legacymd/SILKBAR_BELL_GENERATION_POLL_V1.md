# SilkBar Bell Generation Poll V1

**Status:** Implemented — SilkBar now polls OP_BELL_SUBSCRIBE before OP_BELL_LIST.

**Date:** 2026-05-06

---

## Summary

Updated SilkBar's Bell presence polling to use the new `OP_BELL_SUBSCRIBE` generation
counter check. Instead of calling `OP_BELL_LIST` every 2 seconds unconditionally,
SilkBar first calls `OP_BELL_SUBSCRIBE` to get the current generation counter.
If the generation differs from the cached value, `OP_BELL_LIST` is called for full
aggregate lane counts. If unchanged, the LIST call is skipped entirely.

No ABI changes. No new kernel dependencies. Backward-compatible fallback to LIST.

---

## 1. State Added

```rust
let mut bell_gen_cached: u64 = 0;     // Cached Bell generation (0 forces first LIST)
let mut bell_pending_list: bool = false; // True when LIST is enqueued, awaiting reply
```

- `bell_gen_cached` starts at 0, which guarantees the first SUBSCRIBE reply (gen ≥ 1)
  triggers a LIST poll.
- `bell_pending_list` prevents concurrent LIST/SUBSCRIBE calls and disambiguates
  reply type in the message handler.

---

## 2. Poll Flow (every ~2 seconds)

```
 ┌─────────────────────────────────────────────┐
 │ uptime_seconds % 2 == 0 && !bell_pending_list│
 └──────────┬──────────────────────────────────┘
            │
            ▼
 ┌──────────────────────┐
 │ pdx_call(BELL,       │
 │   OP_BELL_SUBSCRIBE) │
 └──────────┬───────────┘
            │
     ┌──────┴──────┐
     │ Err(e)      │ Ok
     ▼             ▼
 ┌──────────┐     SUBSCRIBE enqueued.
 │ Fallback │     Reply arrives async
 │ to LIST  │     via msg.type_id=1.
 └──────────┘     Handled in msg loop
     │             (else branch).
     ▼
 ┌──────────┐
 │ LIST     │
 │ enqueued │
 │ pending  │
 └──────────┘
```

---

## 3. Reply Handling (message loop, type_id=1, caller_pd=1)

### SUBSCRIBE reply (else branch)

| Condition | Action |
|-----------|--------|
| `gen == u64::MAX` | Denied — fall back to LIST |
| `gen != bell_gen_cached` | Update cache, call LIST, set `bell_pending_list = true` |
| `gen == bell_gen_cached` | No change — skip update |

### LIST reply (if branch)

| Condition | Action |
|-----------|--------|
| `bell_pending_list == true` | Forward packed counts to sexdisplay as `SetBellPresence`, clear pending flag |

---

## 4. Fallback Behavior

If SUBSCRIBE fails (ERR_CAP_INVALID — Bell not available) or returns `u64::MAX` (denied),
SilkBar falls back to the old LIST-only polling path. If LIST also fails, it sends
dim state (all zeros) to sexdisplay.

This ensures backward compatibility if SUBSCRIBE is ever removed or if the Bell
allowlist differs between SUBSCRIBE and LIST in the future.

---

## 5. Markers Added

| Marker | Budget | When |
|--------|--------|------|
| `[silkbar.bell.gen.reply] gen=N changed=0/1` | 8 | SUBSCRIBE reply received |
| `[silkbar.bell.gen.fallback] reason=denied` | 8 | SUBSCRIBE denied, falling back to LIST |
| `[silkbar.bell.gen.fallback] reason=cap_err err=X` | 8 | SUBSCRIBE cap error, falling back to LIST |

---

## 6. Files Changed

| File | Change |
|------|--------|
| `servers/silkbar/src/main.rs` | Added SUBSCRIBE import, cached generation state, reply handler with state machine, updated poll block with fallback |
| `docs/handoff/SILKBAR_BELL_GENERATION_POLL_V1.md` | This handoff document |

---

## 7. Build Result

```
[SEXOS ENTRYPOINT] success
```

No new errors. Pre-existing `static mut` budget counter warnings unchanged.

---

## 8. STOP FIRST Compliance

| Condition | Check | Stop? |
|-----------|-------|-------|
| Display/framebuffer ownership | Not touched | ❌ Not triggered |
| Bell LIST packed format | Not changed | ❌ Not triggered |
| Poll interval | Still every 2s (`uptime_seconds % 2 == 0`) | ❌ Not triggered |
| Push IPC/blocking | Not added | ❌ Not triggered |
| Allowlist | Not changed | ❌ Not triggered |
| sex-pdx | Not changed | ❌ Not triggered |
| Bell code | Not changed | ❌ Not triggered |

---

*End of SILKBAR_BELL_GENERATION_POLL_V1.md*
