# B3: Deterministic Tiling over Scene/Frame/Tab

**Status:** Approved
**Commit:** `40d9515`
**Build:** Passed (ISO produced)
**Behavior:** Unchanged for normal cases; hardened filtering prevents tiling invalid surfaces.

## Purpose

Implement deterministic tiling over the B1/B2 Scene/Frame/Tab model with
lifecycle-aware filtering. Active scene only. No renderer/ABI/kernel changes.

## Changes to `servers/silk-shell/src/main.rs`

### New function: `tile_active_scene_frames()`

Replaces `tile_visible_frames()` at all call sites. Keeps the same layout rules
but adds B3 lifecycle filtering and proof markers.

**Filtering (applied per-frame, in order):**
| Filter | Skip marker | Reason |
|--------|-------------|--------|
| Not in active scene | — | scene_id check |
| FRAME_FLAG_MINIMIZED | — | hidden via 0xEE |
| FRAME_FLAG_ZOOMED | — | full content area via layout_maximize() |
| `!surface_is_alive()` | `[tiling.frame.skip] reason=dead` | dead surface |
| `is_tombstoned()` | `[tiling.frame.skip] reason=tombstoned` | tombstoned surface |
| `!surface_is_lifecycle_focusable()` | `[tiling.frame.skip] reason=lifecycle` | Closing/Destroyed/Hidden/Allocated |
| `!focus_ref_is_current()` | `[tiling.frame.skip] reason=generation` | stale generation |

**Layout rules (unchanged from V1):**
| Frame count | Layout |
|-------------|--------|
| 1 | Full content area |
| 2 | Left/right vertical split |
| 3 | Master left + two-stack right |
| 4 | 2x2 grid |
| 5+ | Stacked rows (full width, equal height) |

### Focus validation after tiling

After tiling completes, the focused surface is validated:
- Must be alive, not tombstoned, lifecycle-focusable, and in active scene
- If invalid: `[tiling.focus.clear]` is emitted, focus moves to first tiled surface
- If no candidates: focus clears to 0

### Call sites updated

All 15 call sites of `tile_visible_frames()` now call `tile_active_scene_frames()`:
- Frame/tab attach (Linen, Quil)
- Close/tombstone/destroy path
- Minimize and restore
- Scene switch
- Zoom toggle
- Silkbar/silkspacer actions
- Boot init
- Manual layout reset (F-key handler)

### Old function preserved

`tile_visible_frames()` is kept at line 810 for reference. No call sites remain.

## Proof Markers

| Marker | Location | Trigger |
|--------|----------|---------|
| `[tiling.active_scene.start]` | tile_active_scene_frames() | Start of tiling |
| `[tiling.frame.skip] reason=dead` | Frame filter | Dead surface |
| `[tiling.frame.skip] reason=tombstoned` | Frame filter | Tombstoned surface |
| `[tiling.frame.skip] reason=lifecycle` | Frame filter | Non-focusable lifecycle |
| `[tiling.frame.skip] reason=generation` | Frame filter | Stale generation |
| `[tiling.frame.apply]` | Per-frame geometry | 0xEC upsert sent |
| `[tiling.focus.clear]` | After tiling | Focus invalid after tiling |
| `[tiling.done]` | End of tiling | Tiling complete |

## Invariants

1. Tiling only operates on the active scene
2. Minimized, Zoomed, Closing, Tombstoned, Destroyed, Hidden surfaces are excluded
3. Dead surfaces and stale-generation tabs are excluded
4. Only `0xEC` (upsert geometry) is used for position/size — no new opcodes
5. After tiling, focus survives only if B2 guards still pass
6. All frame geometry is tracked in shell-local shadow state before 0xEC send

## Deferred

- B4: Tab strip + hover/frame-light behavior
- C1: Atlas (only after B3/B4 stable)
- Dynamic layout policies (user-configurable tiling modes)

## Dependencies

- **Requires:** B1 (Scene/Frame/Tab model), B2 (active-scene focus guards)
- **Blocks:** B4 (tab strip), C1 (Atlas)
