# SILK_SHELL_TILER_PATH_DEDUPE_V1

Date: 2026-05-06

## Tiler Path Audit
Found two tiler implementations in `servers/silk-shell/src/main.rs`:
- `tile_visible_frames()`
- `tile_active_scene_frames()`

Callsite audit showed lifecycle operations already call `tile_active_scene_frames()` directly. `tile_visible_frames()` had duplicate logic and drift risk.

## Canonical Path Chosen
Canonical tiler: `tile_active_scene_frames()`.
Reason: it already includes lifecycle-safe filtering and active-scene semantics, and is the path wired by current lifecycle event callsites.

## Behavior Differences Found
`tile_visible_frames()` lacked the full lifecycle/stale-generation filtering and had independent marker behavior, making it drift-prone.

## Dedupe Applied
- Replaced `tile_visible_frames()` body with delegation:
  - emits `[shell.tile.delegate] from=tile_visible_frames to=tile_active_scene_frames`
  - calls `tile_active_scene_frames()`
- Normalized skip marker coverage in canonical path by adding `[shell.tile.skip_dead]` alongside existing `[tiling.frame.skip]` for:
  - dead
  - tombstoned
  - lifecycle-non-focusable
  - stale generation

## Marker Set
Canonical path now consistently emits:
- `[shell.tile.begin]`
- `[shell.tile.apply]`
- `[shell.tile.skip_dead]`
- `[shell.tile.after_lifecycle]`
- `[shell.tile.reject]`
- `[shell.tile.delegate]` (legacy entrypoint)

## Build
- `./scripts/entrypoint_build.sh` passes.

## Remaining Gaps
- Remaining shell polish is runtime interaction consistency and marker noise consolidation.
- No kernel/ABI/renderer ownership changes required for this dedupe.
