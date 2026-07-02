# LINEN_NONBLOCKING_OPEN_IMPL_V1

Status: **PASS** — Phase 1 implemented and verified
Date: 2026-05-14
Attempts: 1
Implementation: **silkshell-only, ~40 lines**

## Summary

Implemented the safe nonblocking Linen open design from
LINEN_BLOCKING_OPEN_ARCH_REVIEW_V1.md Candidate A+C.

All dispatch paths (palette, keyboard, mesh, OP_LINEN_OPEN_INTENT) now use
`linen_paint_surface_fast()` which renders from already-seeded local data
without calling `linen_sync_reply()` or `linen_fetch_remote_snapshot()`.

## Changes (5 hunks, 1 file)

### Change 1: `linen_paint_surface_fast()` — new function
Added after `linen_paint_surface()` (~line 1905).
- Renders from current LINEN_OBJECTS (seeds or remote)
- No `linen_sync_reply()`, no `linen_fetch_remote_snapshot()`
- Pure fire-and-forget `pdx_call` to SLOT_DISPLAY
- Emits `[linen.fast_paint] sid=N objects=N ok=1 reason=seeds_only|remote_ready`

### Change 2: `open_linen_in_active_scene()` — replace blocking paint
- Line ~8031: duplicate-focus path → `linen_paint_surface_fast()` + `[linen.open.nonblocking] path=duplicate_focus`
- Line ~8084: open-scene path → `linen_paint_surface_fast()` + `[linen.open.nonblocking] path=open_scene`

### Change 3: `mesh_focus_linen_at_selected_fact()` — remove redundant paint
- Removed explicit `linen_paint_surface()` call (line ~3213)
- `open_linen_in_active_scene()` already calls fast paint — no double render
- Marker: `[linen.open.nonblocking] path=mesh_detail ok=1 reason=no_redundant_paint`

### Change 4: OP_LINEN_OPEN_INTENT — fire-and-forget
- Removed `linen_sync_reply()` blocking wait (4 lines)
- Linen always replies 0 (accepted) — direct route to Quil is safe
- Marker: `[linen.sync_reply.skip] path=OP_LINEN_OPEN_INTENT reason=fire_and_forget`
- Marker: `[linen.open.nonblocking] path=intent ok=1 reason=fire_and_forget`

### Change 5: Palette FocusLinen guard — removed
- Removed `COMMAND_PALETTE_DAILY_PROOF_ACTIVE` guard (8 lines)
- Now safe because `open_linen_in_active_scene()` uses fast paint

## What Was NOT Changed
- `linen_paint_surface()` — unchanged (used by event-loop deferred path only)
- `linen_fetch_remote_snapshot()` — unchanged (called from event-loop only)
- `linen_sync_reply()` — unchanged (called from event-loop only)
- Event-loop deferred paint at line ~14863 — unchanged (calls full paint)
- No ABI, kernel, sexdisplay, sexusb, Quil changes

## Proof Results

Build: `SEXOS_LINEN_NONBLOCKING_OPEN_PROOF=1 ./scripts/entrypoint_build.sh`

### Markers emitted (proof build)
| Marker | Value | Meaning |
|--------|-------|---------|
| `[linen.fast_paint]` | sid=200 objects=6 ok=1 reason=seeds_only | Fast paint used, 6 seed objects |
| `[linen.open.nonblocking]` | path=duplicate_focus ok=1 reason=fast_paint | Duplicate path nonblocking |
| `[linen.open.nonblocking]` | path=intent ok=1 reason=fire_and_forget | Intent path nonblocking |
| `[linen.sync_reply.skip]` | path=OP_LINEN_OPEN_INTENT reason=fire_and_forget | Sync reply skipped |
| `[linen.nonblocking.proof]` | stage=1-5 all ok=1 | All proof stages pass |
| `[linen.nonblocking.proof.done]` | ok=1 | Proof complete |

### Correct ordering verified
```
[linen.fast_paint] ... seeds_only    ← dispatch renders seeds FIRST
[linen.object_list.render]           ← seed objects visible
... (later, async in event loop) ...
[linen.remote.snapshot.begin]        ← full fetch runs asynchronously
```

### Faults
faults=0 (#PF, #GP, panic, KERNEL PANIC all zero)

### Baseline (no flag)
Zero behavior change — 0 fast_paint/nonblocking markers, 0 faults.
Event-loop deferred path still uses full `linen_paint_surface()` with remote fetch.

## Files Changed
- `servers/silk-shell/src/main.rs` — 5 hunks, ~40 lines
- `docs/handoff/LINEN_NONBLOCKING_OPEN_IMPL_V1.md` — created (this document)

## Related Handoffs
- `docs/handoff/LINEN_BLOCKING_OPEN_ARCH_REVIEW_V1.md` — design document
