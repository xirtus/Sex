# LINEN_BLOCKING_OPEN_ARCH_REVIEW_V1

## Status: PASS — Design complete, Phase 1 implementation safe
Date: 2026-05-14
Attempts: 1
Implementation: **NONE** — audit and design only

---

## Current Blocker Summary

`linen_paint_surface()` (silk-shell:1889) blocks the event loop on first call:
it calls `linen_fetch_remote_snapshot()` (silk-shell:1813), which spins in
`linen_sync_reply()` (silk-shell:1774) up to 64 times waiting for Linen PD replies.

`linen_sync_reply()` is an unbounded `loop { pdx_listen_raw(0) }`. HID events
are drained in-loop (mouse stays alive) but the event loop itself does not
progress. All dispatchers that call `open_linen_in_active_scene()` or
`linen_paint_surface()` directly inherit this block.

After first paint, `LINEN_REMOTE_FETCHED = true` (silk-shell:1215) and all
subsequent `linen_paint_surface()` calls are pure fire-and-forget `pdx_call`
to sexdisplay (0xEF rects). The block only occurs once per boot.

**Root cause**: dispatch-path callers (keyboard, mesh, palette) can trigger the
first paint before the event-loop deferred paint at line 14847 runs.

---

## Files Inspected

| File | Lines | Purpose |
|------|-------|---------|
| `servers/silk-shell/src/main.rs` | 1774-1800 | `linen_sync_reply` |
| `servers/silk-shell/src/main.rs` | 1813-1886 | `linen_fetch_remote_snapshot` |
| `servers/silk-shell/src/main.rs` | 1889-1899 | `linen_paint_surface` |
| `servers/silk-shell/src/main.rs` | 1585-1692 | `linen_render_object_list` |
| `servers/silk-shell/src/main.rs` | 1698-end | `linen_render_static_ui` |
| `servers/silk-shell/src/main.rs` | 1299-1309 | `linen_object_table_init` (seed) |
| `servers/silk-shell/src/main.rs` | 7948-7995 | `ensure_linen_frame` |
| `servers/silk-shell/src/main.rs` | 8000-8073 | `open_linen_in_active_scene` |
| `servers/silk-shell/src/main.rs` | 8078-8098 | `focus_or_open_linen` |
| `servers/silk-shell/src/main.rs` | 3193-3198 | `mesh_focus_linen_at_selected_fact` |
| `servers/silk-shell/src/main.rs` | 10752-10804 | `palette_execute_selected` FocusLinen |
| `servers/silk-shell/src/main.rs` | 14838-14850 | event-loop deferred paint |
| `servers/silk-shell/src/main.rs` | 15795-15821 | OP_LINEN_OPEN_INTENT dispatch |

---

## Exact Call Graph (current)

### Chain 1 — Event Loop Deferred Paint (safe, runs once)
```
main loop (line 14838)
  LINEN_PAINT_RUN guard (once)
  if SEXUSB_SYNTHETIC_SLOT2 set → SKIP (LINEN_REMOTE_FETCHED stays false!)
  else → linen_paint_surface()          ← MAY BLOCK (first time only)
    if !LINEN_REMOTE_FETCHED:
      LINEN_REMOTE_FETCHED = true
      linen_fetch_remote_snapshot()     ← BLOCKS up to 64× linen_sync_reply
    else:
      linen_render_*()                  ← pure pdx_call to SLOT_DISPLAY, SAFE
```

### Chain 2 — Palette FocusLinen
```
palette_execute_selected() (line 10752)
  FocusLinen branch (line 10785)
    if COMMAND_PALETTE_DAILY_PROOF_ACTIVE → SKIP (guarded during proof)
    else → open_linen_in_active_scene()  ← MAY BLOCK
      linen_paint_surface() (line 8015 or 8067)
        if !LINEN_REMOTE_FETCHED → BLOCKS
```

### Chain 3 — F8 ToggleLinen / SurfaceAction keyboard
```
handle_hid_event()
  SurfaceAction::ToggleLinen
    focus_or_open_linen()
      open_linen_in_active_scene()       ← MAY BLOCK
        linen_paint_surface()
          if !LINEN_REMOTE_FETCHED → BLOCKS
```

### Chain 4 — Mesh Detail Enter → Linen Focus
```
handle_hid_event() → mesh keyboard (line 15680)
  mesh_focus_linen_at_selected_fact(&fact) (line 3193)
    open_linen_in_active_scene()         ← MAY BLOCK (paint call inside)
    linen_paint_surface()                ← REDUNDANT second call, MAY BLOCK
```
Note: **double call** — `open_linen_in_active_scene()` already calls `linen_paint_surface()`
at lines 8015 and 8067. The explicit call at line 3197 is redundant.

### Chain 5 — OP_LINEN_OPEN_INTENT (keyboard, PrintScreen path)
```
handle_hid_event() (line 15807)
  pdx_call(SLOT_LINEN, OP_LINEN_OPEN_INTENT, obj_id, idx, 0)  ← fire
  linen_sync_reply()   (line 15811)                             ← BLOCKS once
  if reply==0: open_linen_object_in_quil(obj_id)               ← LOCAL, SAFE
```

---

## Safe/Unsafe Helper Table

| Helper | File:Line | Wait | Bounded | HID Drain | F-and-F Safe |
|--------|-----------|------|---------|-----------|--------------|
| `linen_sync_reply()` | 1774 | `loop pdx_listen_raw` | **NO** | Yes | No |
| `linen_fetch_remote_snapshot()` | 1813 | ×64 sync_reply | **NO** | Indirect | No |
| `linen_paint_surface()` | 1889 | once-only fetch | Once/boot | Indirect | **No (first call)** |
| `linen_render_object_list()` | 1585 | none | YES | N/A | **YES** |
| `linen_render_static_ui()` | 1698 | none | YES | N/A | **YES** |
| `open_linen_in_active_scene()` | 8000 | via paint | Once/boot | Indirect | **No (first call)** |
| `ensure_linen_frame()` | 7948 | none | YES | N/A | **YES** |
| `focus_or_open_linen()` | 8078 | via paint | Once/boot | Indirect | **No (first call)** |
| `open_linen_object_in_quil()` | 2236 | none | YES | N/A | **YES** |
| `linen_object_table_init()` | 1299 | none | YES | N/A | **YES** |
| `send_frame_tab_info()` | 12918 | none (pdx_call fire) | YES | N/A | **YES** |
| `pdx_call(SLOT_DISPLAY, 0xEF/0xEC/...)` | various | fire-and-forget | YES | N/A | **YES** |

**Key finding**: `linen_render_object_list()` and `linen_render_static_ui()` are
pure fire-and-forget `pdx_call` to SLOT_DISPLAY. No `linen_sync_reply()`.
Seeds from `LINEN_SEED_OBJECTS` are populated at boot via `linen_object_table_init()`.
The ONLY blocking path is `linen_fetch_remote_snapshot()` inside `linen_paint_surface()`.

---

## Candidate Evaluation

### Candidate A — Split paint into fast/full variants ✓ RECOMMENDED
Split `linen_paint_surface()` into:
- `linen_paint_surface_fast()`: renders from existing `LINEN_OBJECTS` (seeds or remote),
  no fetch, no `linen_sync_reply()`. Used in all dispatch paths.
- `linen_paint_surface_full()` (or keep current `linen_paint_surface`): fetches remote
  if not yet done. Used only from event-loop deferred path (line 14847).

Result: All dispatch paths (keyboard, mesh, palette, F8) render immediately using
seed data. Event-loop does the actual fetch asynchronously (first iteration).

Patch size: ~15 lines in silk-shell only. No ABI. No kernel. No PDX helpers.

### Candidate B — Queue pending intent in shell ✗ OVERKILL
Requires a shell-local state machine and deferred drain. Phase 2 only, not needed
for Phase 1 since Candidate A eliminates the block entirely for all dispatch paths.

### Candidate C — Fire-and-forget OP_LINEN_OPEN_INTENT ✓ COMPANION TO A
Drop `linen_sync_reply()` from Chain 5 (line 15811). Linen's `handle_open_intent`
always replies 0 (accepted). Call `open_linen_object_in_quil(obj_id)` directly
without waiting. No reply needed for correctness.

Patch: remove 3 lines at 15810-15816, replace with 1 direct call. Safe.

### Candidate D — Bounded timeout wrapper ✗ SKIP
Requires modifying `linen_sync_reply` or `pdx_listen_raw`. STOP FIRST boundary.
Not needed given Candidates A+C.

---

## Smallest Recommended Patch (Phase 1)

Single file: `servers/silk-shell/src/main.rs`

### Change 1: Add `linen_paint_surface_fast()` after line 1899
```rust
/// Fast paint path: renders from current LINEN_OBJECTS (seeds or remote) without
/// blocking fetch. Safe for all dispatch paths (keyboard, mesh, palette).
/// Falls through to render helpers which are pure pdx_call fire-and-forget.
unsafe fn linen_paint_surface_fast() {
    serial_println!("[linen.paint.fast] remote_fetched={}", LINEN_REMOTE_FETCHED as u8);
    if linen_object_count() == 0 {
        linen_render_static_ui();
    } else {
        linen_render_object_list();
    }
}
```

### Change 2: Replace `linen_paint_surface()` calls in dispatch paths

In `open_linen_in_active_scene()` (lines 8015 and 8067):
```rust
// line 8015: was linen_paint_surface();
linen_paint_surface_fast();
// line 8067: was linen_paint_surface();
linen_paint_surface_fast();
```

In `mesh_focus_linen_at_selected_fact()` (line 3197):
```rust
// Remove: linen_paint_surface();
// Reason: open_linen_in_active_scene() already calls linen_paint_surface_fast().
// The second call here is redundant and was the double-block risk.
```

### Change 3: Remove `linen_sync_reply()` from OP_LINEN_OPEN_INTENT path

Replace lines 15810-15816:
```rust
// was:
// let reply = linen_sync_reply();
// if reply == 0 {
//     open_linen_object_in_quil(obj_id);
//     serial_println!("[linen.open_intent.quil.open] id={} idx={} ok=1", obj_id, idx);
// } else {
//     serial_println!("[linen.open_intent.quil.open] id={} idx={} ok=0 err={}", obj_id, idx, reply);
// }
// replace with:
open_linen_object_in_quil(obj_id);
serial_println!("[linen.open_intent.quil.open] id={} idx={} ok=1 path=fire_and_forget", obj_id, idx);
```

### Change 4 (optional): Remove FocusLinen proof guard in palette
After the above changes, `open_linen_in_active_scene()` no longer blocks.
The `COMMAND_PALETTE_DAILY_PROOF_ACTIVE` guard at line 10786 can be removed.
This enables palette FocusLinen to work during proof runs.

---

## STOP-FIRST Boundaries

Phase 1 has NO stop-first boundaries. All changes are within `silk-shell/src/main.rs`,
touch no ABI, no PDX helpers, no kernel, no sexdisplay, no sexusb, no Quil.

Phase 2 (true async snapshot) WOULD trigger stop-first for:
1. New reply opcode to distinguish async snapshot replies from other 0x1 replies.
2. Modifying `pdx_call_and_reply` or `pdx_listen_raw` semantics.
3. State machine in main loop requiring new `static mut` message queue.

---

## What Can Be Implemented Without ABI/Kernel Changes

All of Phase 1 (Changes 1-4 above). Confirmed boundaries:
- `linen_render_object_list()` — safe, no PDX block
- `linen_render_static_ui()` — safe, no PDX block
- `ensure_linen_frame()` — safe, local + fire-and-forget to SLOT_DISPLAY
- `linen_object_table_init()` seeds exist at boot — render works before fetch
- `open_linen_object_in_quil()` — safe, local only

---

## Non-Goals

- No async/await framework
- No message queue in shell
- No new PDX opcodes
- No changes to `linen_sync_reply` (keep it for event-loop full fetch)
- No changes to Linen server (already non-blocking server-side)
- No changes to sex-pdx helpers
- No changes to kernel routing
- No changes to sexdisplay surface lifecycle
- No shared-memory or backing-buffer redesign
- Do NOT call `linen_fetch_remote_snapshot()` from dispatch paths

---

## Runtime Proof Markers for Future Patch

```
[linen.paint.fast] remote_fetched=N  ← fast path used, shows whether fetch done
[linen.open.nonblocking] ok=1        ← confirm no sync_reply in dispatch
[linen.open_intent.quil.open] ... path=fire_and_forget  ← Chain 5 fix confirmed
[linen.object_list.render] w=N h=N   ← render from seeds before remote fetch
[linen.remote.snapshot.begin]        ← fetch runs in event-loop (not dispatch)
[linen.remote.snapshot.ok] count=N   ← fetch complete, LINEN_REMOTE_FETCHED=true
```

---

## Acceptance Criteria

1. `open_linen_in_active_scene()` returns <1ms with no `pdx_listen_raw` spin.
2. Palette FocusLinen returns `ok=1` (not `blocking_risk_confirmed`).
3. Mesh detail Enter → Linen focus returns immediately; surface renders seed data.
4. F8 ToggleLinen does not block keyboard event loop.
5. OP_LINEN_OPEN_INTENT dispatch no longer calls `linen_sync_reply()`.
6. `[linen.paint.fast]` appears in serial log on first keyboard/palette/mesh open.
7. `[linen.remote.snapshot.begin]` appears only from event-loop context.
8. `[linen.object_list.render]` appears before `[linen.remote.snapshot.ok]` (seeds rendered first).
9. No regression: keyboard nav, mouse input, surface management unchanged.

---

## Rollback Plan

**Warning:** If the patch causes display regression or surface lifecycle issues,
revert is surgical: in `open_linen_in_active_scene()` restore both calls from
`linen_paint_surface_fast()` back to `linen_paint_surface()`, and restore the
`linen_sync_reply()` block in Chain 5. Single file, 4 hunks.

Backup before patch: `cp servers/silk-shell/src/main.rs servers/silk-shell/src/main.rs.bak-$(date +%Y%m%dT%H%M%S)`

---

## Is Implementation Safe Next?

**YES — Phase 1 is safe to implement now.**

Conditions met:
- Seeds populated at boot (non-blocking render guaranteed before fetch)
- Both render helpers are pure fire-and-forget
- No ABI changes needed
- No STOP FIRST boundaries triggered
- `linen_sync_reply()` remains for event-loop full-fetch (backward compatible)
- Rollback is trivial (4 line-level hunks in one file)

---

## Future Implementation Prompt

```
MISSION: LINEN_NONBLOCKING_OPEN_PHASE1_V1

Based on LINEN_BLOCKING_OPEN_ARCH_REVIEW_V1.md, implement Phase 1 patch.

File: servers/silk-shell/src/main.rs

Step 1: After linen_paint_surface() at line 1889-1899, add linen_paint_surface_fast():
  - No linen_fetch_remote_snapshot() call
  - Calls linen_render_static_ui() or linen_render_object_list() directly
  - Emits [linen.paint.fast] marker

Step 2: In open_linen_in_active_scene() (lines 8015, 8067):
  - Replace both linen_paint_surface() calls with linen_paint_surface_fast()

Step 3: In mesh_focus_linen_at_selected_fact() (line 3197):
  - Remove the linen_paint_surface() call (redundant; open_linen_in_active_scene already paints)

Step 4: In OP_LINEN_OPEN_INTENT dispatch (lines 15810-15816):
  - Remove linen_sync_reply() call and if/else block
  - Replace with direct open_linen_object_in_quil(obj_id) call
  - Emit [linen.open_intent.quil.open] with path=fire_and_forget

Step 5 (optional): Remove COMMAND_PALETTE_DAILY_PROOF_ACTIVE guard at palette line 10786.

STOP FIRST if any change touches: ABI, sex-pdx, kernel, sexdisplay, sexusb, Quil.
BACKUP before edit.
EMIT proof markers.
NO new static mut globals beyond what already exist.
NO linen_sync_reply() in any dispatch path after this patch.
```

---

## Files Changed

`docs/handoff/LINEN_BLOCKING_OPEN_ARCH_REVIEW_V1.md` — created (this document).
No source files modified.

## Related Handoffs

- `docs/handoff/LINEN_SAFE_NONBLOCKING_OPEN_PLAN_V1.md` — prior V1 plan (confirmed accurate, line numbers updated here)
