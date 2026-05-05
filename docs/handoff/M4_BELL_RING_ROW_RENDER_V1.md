# M4: Bell Ring Row Rendering

**Status:** Complete
**Date:** 2026-05-05
**Purpose:** Wire the already-written Bell event list render function into the
two refresh paths: (1) when Bell is opened, and (2) when a new event is recorded
while Bell is visible.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║                    PASS_M4                                   ║
╠══════════════════════════════════════════════════════════════╣
║ open_bell_in_active_scene:     WIRED (replace single fill)   ║
║ bell_emit_object_link_event:   WIRED (conditional refresh)   ║
║ Visibility gate:               bell_is_visible_in_active_    ║
║                                 scene() helper                ║
║ Read-only render:              NO_RING_MUTATION               ║
║ Boundaries:                    INTAKT                         ║
║ Build:                         PASS                           ║
╚══════════════════════════════════════════════════════════════╝
```

## Wiring Points

### 1. `open_bell_in_active_scene()` — Replace placeholder fill

**Before (old single-fill placeholder):**
```rust
pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_BELL_PLACEHOLDER, 0,
    (BELL_PLACEHOLDER_COLOR as u64) << 32 | ((SURFACE_204_H as u64) << 16) | SURFACE_204_W as u64);
```

**After:**
```rust
bell_render_event_list();
```

The new function draws a header bar (BELL_PLACEHOLDER_COLOR at rect_index=0) plus
up to 7 event rows with colors derived from `bell_row_color()` (which maps to
`linen_kind_color()` for `ObjectLinkedToBuffer` events).

### 2. `bell_emit_object_link_event()` — Conditional refresh after write

Added after `bell_record_event()` and `[bell.ring.done]` marker:

```rust
if bell_is_visible_in_active_scene() {
    serial_println!("[bell.render.refresh] reason=visible_after_link ...");
    bell_render_event_list();
}
```

Only fires when:
- Bell frame exists in active scene
- Bell frame is not minimized
- Bell surface is alive (surface_is_alive)

Does NOT fire on invalid/reject paths (object missing, buffer mismatch).

## New Helper

### `bell_is_visible_in_active_scene() -> bool`

Iterates `FRAMES` checking:
1. Frame ID == BELL_FRAME_ID
2. scene_id == ACTIVE_SCENE_IDX
3. Not minimized (FRAME_FLAG_MINIMIZED == 0)
4. Surface is alive via `surface_is_alive()`

Returns `true` only when all four conditions hold. No focus changes, no lifecycle
changes, no side effects.

## Proof Markers

| Marker | Location | Description |
|--------|----------|-------------|
| `[bell.event_list.render]` | `bell_render_event_list()` start | Render begin, w/h/count |
| `[bell.event_list.row]` | Per event in iterator | Event row emitted (id, kind, object_id, buffer_id) |
| `[bell.event_list.skip]` | Row count ≥ MAX | Row skipped (max_rows) |
| `[bell.row_visual.rect]` | Per fill rect sent | Rect index, event id, kind, color |
| `[bell.row_visual.skip]` | Rect budget exhausted | Rect skipped (rect_budget) |
| `[bell.event_list.done]` | Render complete | Final count, rows, rects |
| `[bell.render.refresh]` | Conditional refresh | Refresh triggered after link event |
| Existing `[bell.ring.*]` | Unchanged | All existing markers preserved |
| Existing `[bell.event.*]` | Unchanged | All existing markers preserved |

## Iteration Order

Newest-first: `bell_for_each_event()` iterates backwards from
`(write_index - 1) % BELL_RING_CAP`. The most recent event appears as row 0
(nearest the header), oldest visible event appears as row 6 (or fewer).

## Refresh Behavior

| Path | Refresh? | Condition |
|------|----------|-----------|
| Open Bell (PageDown) | ✅ Full render | Always on open |
| Focus existing Bell | ❌ No render | Duplicate guard (surface already visible) |
| Link event while Bell hidden | ❌ No render | `bell_is_visible_in_active_scene()` = false |
| Link event while Bell visible | ✅ Full render | After `bell_record_event()` succeeds |
| Rejected link (invalid) | ❌ No render | Validation fails before record |

## Boundaries

| Area | Status |
|------|--------|
| kernel/ | ✅ CLEAN |
| crates/sex-pdx/ | ✅ CLEAN |
| servers/sexdisplay/ | ✅ CLEAN |
| servers/bell/ | ✅ CLEAN |
| PDX ABI/opcodes | ✅ CLEAN |
| WINDOWS Vec | ✅ CLEAN |
| Ring mutation during render | ✅ NONE (read-only) |
| Bell PD creation | ✅ NONE |
| New event kinds | ✅ NONE |
| Queue semantics | ✅ NONE (event memory only) |

## Build Result

```
[SEXOS TRACE] stage=package_iso
xorriso : UPDATE :      19 files added in 1 seconds
ISO image produced: 1608 sectors
[SEXOS ENTRYPOINT] success
```

**Build: PASS** — ISO produced successfully.

## Changed Files

- `servers/silk-shell/src/main.rs` — 34 insertions, 5 deletions

## Commit

```
git commit -m 'feat(bell): wire event row rendering'
```

## Next Steps

**M5: Rapid audit of M4** — close the Bell ring rendering milestone.
After M5: return to real feature work selection.
