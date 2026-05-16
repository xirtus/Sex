# FRAME_LIGHT_CLOSE_ENABLEMENT_V1

## Root Cause
The red-close behavior was blocked in userland policy and proof wiring after prior revert:
- `silk-shell` app registry marked all app surfaces `closeable: false`.
- frame-light proof markers were hardcoded to `red=disabled` / `close_allowed=0`.
- `sexdisplay` rendered close light with permanently dim disabled alpha, ignoring per-frame close-allow model state.

This produced persistent gray/red-disabled visual state even when close FSM code existed.

## Ownership Boundary
- `silk-shell` computes close policy (`close_allowed`) and owns lifecycle FSM.
- `sexdisplay` only renders based on model/chrome flags and keeps bounded framebuffer writes.
- No kernel/ABI/sex-pdx changes.

## Files Changed
- `servers/silk-shell/src/main.rs`
- `servers/sexdisplay/src/main.rs`
- `scripts/daily_driver_master_gate.sh`

## Minimal Diff Summary
### silk-shell
1. Restored disposable close policy in app registry:
- set `closeable: true` for disposable app frames (Linen/Quil/Mesh/Collar/Bell/Spindle/Browser).
- kept system/protected surface classes non-closeable (cursor/launcher/status/clock/bell/system panels remain blocked in `is_closeable_surface`).

2. Added helper:
- `frame_close_allowed(frame_id)` derives close permission from active surface policy.

3. Propagated close policy to renderer model:
- `send_frame_tab_info()` now packs `close_allowed` into chrome_flags bit 5.

4. Restored frame-light proofs to model-driven behavior:
- `silk.frame.lights.state` now emits enabled/disabled per frame from policy.
- summary now emits `red_enabled>0` when disposable frames exist.
- keyboard proof emits red close actions with `ok=1 reason=close_allowed` where permitted.
- executes one disposable close path (`close_surface_from_frame_light`) and emits lifecycle/focus markers.

### sexdisplay
1. Renderer now honors close-allowed model bit:
- reads chrome bit 5 (`SURFACE_CHROME_CLOSE_ALLOWED`).
- renders close light bright when allowed; dim only when disallowed.
- preserves bounded framebuffer behavior and existing geometry.

### gate
Updated frame-light gates to enforce restored behavior:
- `frame_lights_stub` PASS requires:
  - `red_enabled>0` for disposable frames,
  - explicit protected-system frame with `close_allowed=0`.
- `frame_lights_keyboard` PASS requires:
  - at least one `light=red action=close ... ok=1 reason=close_allowed`,
  - `frame.light.close.fsm` marker present.
- fail when all red are disabled or required markers are missing.

## Proof Commands
- `./scripts/entrypoint_build.sh`
- `./scripts/run_daily_driver_proof.sh /tmp/sexos_frame_light_close_enablement_v1.log`

## Proof Result
- `FINAL: PASS (123 gates proved, 0 skipped, 0 faults)`
- zero fault markers (`#PF/#GP/panic/fault.kill` absent).

Key evidence from `/tmp/sexos_frame_light_close_enablement_v1.log`:
- `[silk.frame.lights.state] ... red=enabled ... close_allowed=1 ... reason=close_allowed`
- `[silk.frame.lights.state] ... red=disabled ... close_allowed=0 ... reason=protected_system_frame`
- `[silk.frame.lights.summary] ... red_enabled=3 ...`
- `[silk.frame.lights.action] light=red action=close frame=... ok=1 reason=close_allowed`
- `[frame.light.close.fsm] sid=...`
- `[app.lifecycle.transition] ... new=destroyed ...`
- `[focus.clear] sid=... reason=closed_surface`

## Remaining Risks
- Enabling close for core app placeholders (e.g. Linen/Quil/Spindle) is intentional per disposable policy restoration here; if product policy later narrows this set, only `APP_SURFACES.closeable` needs adjustment.
