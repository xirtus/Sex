# SILK_SHELL_MARKER_NOISE_CLEANUP_V1

Date: 2026-05-06

## Marker Audit Summary
Performed shell marker cleanup with zero behavior changes. Focused on removing duplicate success-path logs and budgeting high-frequency markers while preserving invariant/reject diagnostics.

## KEEP_ALWAYS (critical invariant / failure proof)
- Boot truth + reject chain:
  - `[silk-shell.boot.layout.content]`
  - `[silk-shell.boot.layout.reject]`
  - `[silk-shell.boot.surface.bounds]`
  - `[silk-shell.compose.order]`
  - `[silk-shell.boot.surface.hidden]`
  - `[silk-shell.boot.zorder]`
  - `[silk-shell.boot.zorder.reject]`
  - `[silk-shell.boot.ui.ready]`
  - `[silk-shell.boot.reject]`
- Lifecycle safety:
  - `[shell.focus.clear_dead]`
  - `[shell.drag.clear_dead]`
  - `[shell.hover.clear_dead]`
  - `[shell.interact.reject]`
- Tiling integrity:
  - `[shell.tile.reject]`
  - `[shell.tile.skip_dead]`
  - `[shell.tile.after_lifecycle]`
  - `[shell.tile.delegate]`

## BUDGETED (high-value but spam-prone)
- `[shell.interact.hover]` (budget lowered to 8)
- `[shell.interact.drag.move]` (budgeted)
- `[shell.tile.apply]` (now budgeted via `SHELL_TILE_APPLY_BUDGET`)

## REMOVED_OR_MERGED
Removed duplicate noisy success markers in favor of canonical `shell.interact.*`:
- removed `[tiling.active_scene.start]`
- removed `[tiling.frame.apply]`
- removed `[tiling.done]` final summary
- removed `[shell.drag.start]` (kept `[shell.interact.drag.begin]`)
- removed `[shell.drag.move]` and `[shell.drag.send.ok]` (kept `[shell.interact.drag.move]`)
- removed duplicate `[shell.drag.end]` logs (kept `[shell.interact.drag.end]`)

## Grep Map (recommended)
- Boot truth:
  - `grep -E "silk-shell.boot.layout|silk-shell.boot.surface|silk-shell.boot.zorder|silk-shell.boot.ui.ready|silk-shell.compose.order"`
- Lifecycle invalidation:
  - `grep -E "shell\.focus\.clear_dead|shell\.drag\.clear_dead|shell\.hover\.clear_dead|shell\.interact\.reject"`
- Tiling lifecycle:
  - `grep -E "shell\.tile\.(begin|apply|skip_dead|after_lifecycle|reject|delegate)"`
- Runtime interactions:
  - `grep -E "shell\.interact\.(focus|hover|drag\.(begin|move|end)|minimize|restore|scene\.switch|atlas\.select|tile\.return)"`

## Build
- `./scripts/entrypoint_build.sh` passes.

## Remaining Polish Gaps
- Marker volume is reduced but still non-trivial under heavy interaction loops.
- Further reduction could collapse some `shell.interact.*` success markers behind a global debug mode if needed.
