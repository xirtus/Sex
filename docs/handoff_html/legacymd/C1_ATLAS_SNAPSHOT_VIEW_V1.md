# C1: Atlas Snapshot/View

**Status:** Approved
**Commit:** `1a85032`
**Build:** Passed (ISO produced)
**Behavior:** Unchanged for normal mode; Atlas overview now correctly skips lifecycle-invalid frames in snapshot.

## Purpose

Implement Atlas overview snapshot/view over stable B1-B4 Scene/Frame/Tab geometry.
Snapshot and view only. No navigation/focus switching changes.

## Principle

**silk-shell owns Atlas policy/snapshot state. sexdisplay remains pixel renderer only.**
Atlas rendering uses existing 0xEC (overlay surface), 0xEF (fill rect), and 0xEE (hide) ops.
No new display primitives or ABI changes were needed.

## Changes to `servers/silk-shell/src/main.rs`

### 1. `atlas_capture_snapshot()` — lifecycle filtering hardened

The snapshot now filters frames using full B1-B4 lifecycle criteria:

| Skip Reason | Filter | Marker |
|-------------|--------|--------|
| Minimized | `FRAME_FLAG_MINIMIZED` | `[atlas.snapshot.skip] reason=minimized` |
| Dead surface | `!surface_is_alive()` | `[atlas.snapshot.skip] reason=dead` |
| Tombstoned | `is_tombstoned()` | `[atlas.snapshot.skip] reason=tombstoned` |
| Closing/Destroyed/Hidden | `lifecycle_state()` match | `[atlas.snapshot.skip] reason=lifecycle:{state}` |
| Stale generation | `!focus_ref_is_current()` | `[atlas.snapshot.skip] reason=generation` |

Previously only dead/tombstoned were filtered. Now all lifecycle-invalid states are excluded.

### 2. `atlas_toggle()` — view enter/exit markers

- `[atlas.view.enter]` — Atlas mode entered, overlay rendered
- `[atlas.view.exit]` — Atlas mode exited, overlay cleared

### 3. Existing render path (unchanged)

The `atlas_render_stub()` function already draws Atlas overview using:
- `0xEC` — create overlay surface at full content area
- `0xEF` — fill background, cards, frame blocks, selection border
- `0xEE` — hide overlay on exit
- No new opcodes, no sexdisplay changes, no ABI edits

## Proof Markers

| Marker | Location | Trigger |
|--------|----------|---------|
| `[atlas.snapshot.start]` | atlas_capture_snapshot() | Capture begin |
| `[atlas.snapshot.skip] reason=*` | Frame filter loop | Frame excluded from snapshot |
| `[atlas.snapshot.frame]` | Frame filter loop | Frame included in snapshot |
| `[atlas.snapshot.scene]` | Per-scene end | Scene summary with frame count + flags |
| `[atlas.view.enter]` | atlas_toggle() | Atlas mode entered |
| `[atlas.view.exit]` | atlas_toggle() | Atlas mode exited |

## Snapshot Fields (SceneDescriptor)

| Field | Source | Description |
|-------|--------|-------------|
| `scene_id` | Scene index (0..4) | Maps to workspace |
| `label` | `SCENES[].label` | Fixed 16-byte label |
| `flags` | `SCENES[].flags` | SCENE_FLAG_ACTIVE, EMPTY, HAS_FOCUS, etc. |
| `focused_frame_id` | Active scene only | Frame with focus in active scene |
| `frame_count` | Derived from FRAMES | Number of valid frames in scene |
| `frame_ids` | Derived from FRAMES | Fixed-size array of frame IDs |

## Invariants

1. Atlas snapshot excludes all lifecycle-invalid frames (minimized, dead, tombstoned, Closing, Destroyed, Hidden, stale-generation)
2. No new display primitives or ABI changes required (existing 0xEC/0xEF/0xEE ops suffice)
3. sexdisplay never owns scene policy — it only renders what it is told
4. Atlas enter/exit does not change focus, scene, or tiling state
5. B2/B3/B4 invariants are preserved (focus guards, tiling, chrome visibility)

## Deferred

- Atlas navigation/focus switching (arrow keys, Enter to select) — already implemented as pre-C1 feature, kept as-is
- Scene thumbnails (would require new renderer primitive — STOP FIRST)
- Atlas scene management (add/remove/reorder scenes)

## Dependencies

- **Requires:** B1 (Scene/Frame/Tab), B2 (active-scene focus), B3 (tiling), B4 (tab chrome)
- **No new dependencies**

## Renderer/ABI Assessment

**No renderer or ABI changes required.**
The existing 0xEC (upsert geometry), 0xEF (fill rect), and 0xEE (deactivate) ops
provide sufficient primitives for Atlas card-based overview rendering.
