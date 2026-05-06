# SILK_SHELL_RUNTIME_INTERACTION_PROOF_V1

Date: 2026-05-06

## Interaction Paths Audited
Runtime shell interaction paths covered:
- focus set/reject (`try_set_focus`)
- hover update (`update_frame_hover_at`)
- drag begin/move/end
- minimize/restore lifecycle transitions
- scene switch
- Atlas scene select
- return-to-tiler after lifecycle transitions

## Markers Added
Added proof markers with bounded behavior where hot-path:
- `[shell.interact.focus]`
- `[shell.interact.hover]`
- `[shell.interact.drag.begin]`
- `[shell.interact.drag.move]`
- `[shell.interact.drag.end]`
- `[shell.interact.minimize]`
- `[shell.interact.restore]`
- `[shell.interact.scene.switch]`
- `[shell.interact.atlas.select]`
- `[shell.interact.tile.return]`
- `[shell.interact.reject]`

## Invalid Target Guard Verification
Focus guard path now emits explicit reject reasons for:
- nonfocusable
- dead
- tombstoned
- lifecycle-not-focusable
- stale-generation
- inactive-scene

Hover/drag invalidation remains lifecycle-safe through existing clear paths and now has interaction-proof visibility via added markers.

## Canonical Tiler Return Proof
All lifecycle interaction paths in this patch route through canonical tiler and now emit explicit return markers:
- minimize -> `tile_active_scene_frames` + `[shell.interact.tile.return] source=minimize ...`
- restore -> `tile_active_scene_frames` + `[shell.interact.tile.return] source=restore ...`
- scene switch -> `tile_active_scene_frames` + `[shell.interact.tile.return] source=scene.switch`

Atlas select path already triggers scene switch (or active-scene restore path) and now emits `[shell.interact.atlas.select]`.

## Build
- `./scripts/entrypoint_build.sh` passes.

## Remaining Runtime Polish Gaps
- High-frequency drag/hover marker noise could be reduced further if desired.
- Interaction UX tuning (not correctness) remains for edge cases under rapid mixed input.
- No kernel/ABI/renderer ownership changes required for this proof pass.
