# SILK_SHELL_95_CLOSER_NOW_V1

Date: 2026-05-06

## Model Status
Current shell model is coherent and enforced through frame flags + lifecycle + tombstone checks:
- live/visible/mapped/hidden/minimized/closing/tombstoned/destroyed states are represented
- active-scene filtering is enforced in tiling path
- minimized/zoomed frames are excluded from active tiling
- dead/tombstoned/non-focusable targets are rejected

## Tiling Behavior (Active Path)
Primary deterministic tiler is `tile_active_scene_frames()`.
Layout remains deterministic:
- 1 frame: full content rect
- 2 frames: vertical split
- 3 frames: left master + right stack
- 4 frames: 2x2
- 5+ frames: bounded stacked rows

Filtering in this path skips:
- minimized
- zoomed
- dead
- tombstoned
- lifecycle-non-focusable
- stale generation FocusRef

## Hardening Added (This Patch)
Added/normalized bounded markers for lifecycle-safe tiling proof:
- `[shell.tile.begin]`
- `[shell.tile.apply]`
- `[shell.tile.after_lifecycle]`
- `[shell.tile.reject]`
- existing `[shell.tile.skip_dead]` retained in companion tiling path

Invalid-target clearing markers:
- `[shell.focus.clear_dead]`
- `[shell.drag.clear_dead]`
- `[shell.hover.clear_dead]`

Additional behavior hardening:
- `tile_active_scene_frames()` now clears hover via `clear_hover_if_wrong_scene()` when no tileable frames exist.
- `clear_drag_if_dead()` now also clears drag when drag target is no longer lifecycle-focusable.

## Lifecycle Event Wiring
Tiling remains wired on lifecycle transitions already present in code:
- minimize / restore
- unzoom return to tiled
- scene switch
- atlas select/exit/commit
- snapshot restore

No tiling was added to pointer drag motion paths.

## Boot Regression Check
Boot policy from prior fixes remains unchanged:
- Quil focused full-content boot
- Linen hidden-consistent under focused Quil
- composition semantics unchanged (focused surface topmost)

## Build
- `./scripts/entrypoint_build.sh` passes.

## Remaining Gap to 95%
Main remaining gap is not boot or kernel/ABI: it is runtime UX polish and consistency under mixed interactions (rapid scene switches + drag + frame-light actions), plus pruning duplicate/legacy tiler paths (`tile_visible_frames` vs `tile_active_scene_frames`) in a dedicated cleanup pass.
