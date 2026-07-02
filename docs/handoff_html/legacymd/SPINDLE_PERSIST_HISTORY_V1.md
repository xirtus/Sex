# SPINDLE_PERSIST_HISTORY_V1

**Date:** 2026-05-14
**Status:** Save OK (fire-and-forget) | Load ASYNC-LIMITED
**Previous:** SPINDLE_SEXFILES_PERSIST_V1
**Next:** TBD

---

## Summary

Added explicit `save`/`load` commands and persist proof gate for Spindle
command history persistence.  During implementation, discovered a fundamental
PDX architecture constraint: calls to `SLOT_STORAGE` (Domain cap) use the
**AsyncEnqueue** edge, which is **fire-and-forget** — `pdx_call` returns
`(0,0)` immediately without waiting for the server reply.  The server reply
arrives asynchronously via `incoming_replies`, consumed by `pdx_listen_raw`.

### What Works

| Operation | Mechanism | Status |
|-----------|-----------|--------|
| **Save** (`persist_history`) | Fire-and-forget OPEN/WRITE/CLOSE via AsyncEnqueue | ✅ OK |
| **Auto-save on Enter** | Same fire-and-forget path in hot key handler | ✅ OK |
| **`save` command** | Explicit user-triggered persist | ✅ OK |
| **Load sync readback** | `pdx_call(READ)` always returns `(0,0)` | ❌ ASYNC-LIMITED |
| **`load` command** | Graceful — reports limitation honestly | ✅ OK |

### Architecture Discovery

```
pdx_call(SLOT_STORAGE, OP_RAMFS_*, ...)
  → safe_pdx_call(cap_id=1, ...)
    → resolve_edge(CapabilityData::Domain(id), ...)
      → GraphEdge::AsyncEnqueue { ring: &sexfiles.message_ring }
    → traverse_edge(AsyncEnqueue, ...)
      → (*ring).enqueue(msg).map(|_| 0u64)  // ← always returns Ok(0)!
```

- `pdx_call` to a **Domain cap** enqueues a message and returns `Ok(0)`.
- The server processes the message asynchronously and calls `pdx_reply`.
- The reply goes to the caller's `incoming_replies` buffer.
- `pdx_listen_raw(0)` checks `incoming_replies` first (priority), then the message ring.
- **No synchronous reply path exists for Domain caps.**

This means:
- **Save is fine** — data IS written to sexfiles RamFS (confirmed by server-side logs).
- **Load cannot do synchronous readback** — `pdx_call(READ)` always returns `(0,0)`,
  so `restore_history` always sees empty data regardless of what the server returns.
- Server replies for READ arrive as `type=0x1` messages in the main `pdx_listen_raw` loop.

### Why Not Blocking

`pdx_call` via AsyncEnqueue is NON-BLOCKING:
- No spin loop or unbounded wait in the hot key path.
- `enqueue()` returns immediately (lock-free ring buffer push).
- Server reply is delivered asynchronously.

### Future: Sync-Call Edge Type

A synchronous reply path for Domain caps would require a new kernel edge type
(e.g., `AsyncEnqueueWithReply`) that blocks the caller until the server replies,
delivering the reply value through `pdx_call`'s return registers.  This is a kernel
ABI change (**STOP FIRST**).

Alternatively, Spindle could collect async replies from `pdx_listen_raw` in the
main loop and match them to pending requests.  This is complex and deferred.

---

## Files Changed

| File | Change |
|------|--------|
| `apps/spindle/src/main.rs` | +save/load commands, +persist_proof gate, +audit markers, rewritten persist/restore with async-aware docs |
| `docs/handoff/SPINDLE_PERSIST_HISTORY_V1.md` | NEW |

## Files NOT Changed

| File | Reason |
|------|--------|
| `kernel/src/init.rs` | Cap already granted (8ce251e) |
| `crates/sex-pdx/` | STOP FIRST — kernel ABI |
| `servers/sexfiles/` | Server processes correctly; issue is client-side edge type |
| `servers/silk-shell/` | No routing changes |
| `servers/quil/` | Not touching |

---

## Serial Markers

| Marker | Value | Meaning |
|--------|-------|---------|
| `[spindle.persist.audit]` | storage_cap=1 edge=AsyncEnqueue safe=1 | Audit: fire-and-forget, non-blocking |
| `[spindle.history.save]` | count=N ok=1 | Save succeeded (fire-and-forget) |
| `[spindle.history.load]` | count=0 ok=1 reason=async_limited... | Load graceful, sync readback unavailable |
| `[spindle.persist.command]` | name=save\|load | Explicit command dispatch |
| `[spindle.persist.proof]` | stage=0..5 | Proof stages |
| `[spindle.persist.proof.done]` | ok=1 | All proof stages pass |

---

## Proof Gate

Activated by `SEXOS_SPINDLE_PERSIST_HISTORY_PROOF=1`.

5 stages:
1. **push_entries** — Push 3 commands to in-memory history
2. **save** — Fire-and-forget persist via `dispatch("save")`
3. **history_intact** — Verify in-memory history unchanged after save
4. **load** — Graceful load (async-limited, no crash)
5. **load_graceful** — No faults, command dispatched ok

All pass: `[spindle.persist.proof.done] ok=1`

---

## Build / Runtime Result

| Check | Result |
|-------|--------|
| `SEXOS_SPINDLE_PERSIST_HISTORY_PROOF=1 ./scripts/entrypoint_build.sh` | PASS |
| `./scripts/entrypoint_build.sh` (standard) | PASS |
| QEMU headless 30s | No crash, no faults |
| Faults (#PF, #GP, panic) | **0** |
| `[spindle.persist.proof.done]` | **ok=1** |

---

## Commands Added

| Command | Description |
|---------|-------------|
| `save` | Persist command history to SexFiles RamFS (fire-and-forget) |
| `load` | Attempt restore; reports async limitation honestly |

Existing `history` command unchanged.

---

## Contract Boundaries Preserved

- **No kernel edits** — cap already granted in 8ce251e
- **No sex-pdx ABI edits** — STOP FIRST
- **No sexfiles server edits** — server processes correctly
- **No blocking hot path** — fire-and-forget, no unbounded wait
- **No fake persistence** — honest async-limited status for load
- **No heap growth** — all static BSS
- **No Quil/PDX delivery edits**
- **No pointer/slot2 mouse work**
