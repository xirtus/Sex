# SILK_SHELL_UI_READY_ORDER_PROOF_V1

Date: 2026-05-06

## Scope
- `servers/silk-shell/src/main.rs`
- Metadata-only boot readiness proof markers
- No kernel/ABI/slot/opcode changes

## Root Cause Status
- Quil boot-paint gap was already fixed in `QUIL_INITIAL_BOOT_DRAW_V1`.
- This step adds deterministic shell boot-order observability to prove surface lifecycle readiness for Linen/Quil.

## Markers Added
- `[silk-shell.boot.surface.create]`
- `[silk-shell.boot.surface.visible]`
- `[silk-shell.boot.surface.bounds]`
- `[silk-shell.boot.zorder]`
- `[silk-shell.boot.focus]`
- `[silk-shell.boot.ui.ready]`
- `[silk-shell.boot.reject]`

## Build
- `./scripts/entrypoint_build.sh` passes.

## Expected Boot Proof Chain
1. Shell boot create for surface 201 and 200
2. Bounds markers show non-zero rects
3. Visible markers show both surface states
4. Z-order marker confirms deterministic ordering token
5. Focus marker confirms valid focus target (201)
6. UI ready marker confirms surface count and focus

## Next Runtime Check
Use local GUI lane and grep for:
- `silk-shell.boot.surface.create`
- `silk-shell.boot.surface.bounds`
- `silk-shell.boot.surface.visible`
- `silk-shell.boot.zorder`
- `silk-shell.boot.focus`
- `silk-shell.boot.ui.ready`
- `silk-shell.boot.reject`

If UI still appears partial after all proof markers are present, the next dead hop is after shell readiness (likely scene composition/visibility update timing), not Quil boot draw.
