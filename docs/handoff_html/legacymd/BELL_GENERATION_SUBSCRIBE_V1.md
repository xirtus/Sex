# Bell Generation Subscribe V1

**Status:** Implemented — generation counter + OP_BELL_SUBSCRIBE handler.

**Date:** 2026-05-06

---

## Summary

Implemented `OP_BELL_SUBSCRIBE` (0xC5) as a no-kernel-change generation counter poll.
Subscribers call `OP_BELL_SUBSCRIBE` to receive the current `u64` generation counter.
If the generation differs from the cached value, the subscriber calls `OP_BELL_LIST`
to get updated lane-summary aggregates. No push IPC, no blocking, no kernel changes.

---

## 1. Opcode

| Constant | Value | Source |
|----------|-------|--------|
| `OP_BELL_SUBSCRIBE` | `0xC5` | `crates/sex-pdx/src/lib.rs:111` (reserved in Bell V1) |

---

## 2. Generation Counter

```rust
static mut BELL_GENERATION: u64 = 1;

fn bump_generation() {
    unsafe { BELL_GENERATION = BELL_GENERATION.wrapping_add(1); }
}
```

- Initialized to 1 (0 not used; reserved sentinel per Bell convention)
- `wrapping_add` ensures safe wraparound (false positive → one extra LIST poll)
- Single-threaded: no atomic needed

---

## 3. Generation Bump Sites

| Site | Line | Condition | Bump? |
|------|------|-----------|-------|
| NOTIFY enqueue | 515 | `Ok((event_id, _))` returned from queue push | ✅ Always (queue changed) |
| CLOSE dismiss | 730 | `found == true` (event dismissed) | ✅ Always (queue state changed) |
| CLEAR all lanes | 812 | `lane_filter == 0xFF` (queue reset) | ✅ Always (queue changed) |
| CLEAR specific lane | 837 | `lane_filter <= 5 && dismiss_count > 0` | ✅ Only if entries were dismissed |
| MUTE_SENDER add | 871 | `add_mute Ok(())` | ✅ Always (may be idempotent; bounded false positive) |
| MUTE_SENDER remove | 897 | `remove_mute` returned `true` | ✅ Only if actually removed |

**No bump on:** rejected NOTIFY (spam, mute, invalid args), CLOSE not-found, CLEAR invalid lane, MUTE_SENDER full/false.

---

## 4. OP_BELL_SUBSCRIBE Handler

**Protocol:**
```
Request:  msg.type_id = OP_BELL_SUBSCRIBE (0xC5)
          arg0 = unused (0)
Reply:    value = BELL_GENERATION (u64)

Denied:   value = u64::MAX
```

**Security:**

| Property | Implementation |
|----------|---------------|
| Allowlist | Same as LIST: `BELL_LIST_ALLOWLIST = &[3, 6]` (silk-shell, silkbar) |
| Deny behavior | `pdx_reply(caller_pd, u64::MAX)` with `continue` — matches LIST deny pattern |
| Data leaked | Only generation counter (opaque u64). No queue contents, no sender identity. |
| No-op idempotent | Generation unchanged between bumps → subscriber detects no change → skips LIST |

**Markers (budgeted):**

| Marker | Budget | When |
|--------|--------|------|
| `[bell.subscribe.reply] gen=` | 8 | Successful reply to allowed caller |
| `[bell.subscribe.deny] caller_pd=` | 8 | Unauthorized caller rejected |

---

## 5. Usage by Subscribers

```
Cached generation: 0

loop {
    // Poll SUBSCRIBE instead of LIST
    result = pdx_call_checked(BELL_SLOT, OP_BELL_SUBSCRIBE, 0, 0, 0)
    if result != cached_gen {
        cached_gen = result
        // Generation changed — call LIST for full aggregate
        list_reply = pdx_call_checked(BELL_SLOT, OP_BELL_LIST, ...)
        update_display(list_reply)
    }
    sleep(5_seconds)
}
```

Generation-only poll is O(1) (no queue scan). LIST is called only on change.

---

## 6. Files Changed

| File | Change |
|------|--------|
| `servers/sexbell/src/main.rs` | Added `OP_BELL_SUBSCRIBE` import, `BELL_GENERATION` counter, `bump_generation()` helper, 6 bump callsites, SUBSCRIBE handler with markers |
| `docs/handoff/BELL_GENERATION_SUBSCRIBE_V1.md` | This handoff document |

---

## 7. Build Result

```
[SEXOS ENTRYPOINT] success
```

No new warnings.

---

## 8. STOP FIRST Compliance

| Condition | Check | Stop? |
|-----------|-------|-------|
| Kernel ABI change | None — uses existing syscall 29 via `pdx_reply` | ❌ Not triggered |
| Push IPC | Not implemented — poll-only design | ❌ Not triggered |
| Blocking/wait | Not implemented — immediate reply | ❌ Not triggered |
| LIST packed reply change | Not changed | ❌ Not triggered |
| SilkBar polling change | Not changed | ❌ Not triggered |
| Policy table | Not added | ❌ Not triggered |
| Allowlist change | Not broadened | ❌ Not triggered |
| Storage/persistence | None needed | ❌ Not triggered |

---

*End of BELL_GENERATION_SUBSCRIBE_V1.md*
