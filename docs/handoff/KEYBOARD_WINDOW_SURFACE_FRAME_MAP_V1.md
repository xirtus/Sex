# KEYBOARD_WINDOW_SURFACE_FRAME_MAP_V1

## Goal
Resolve keyboard window proof blocker:
`[shell.keyboard.window.proof.skip] reason=no_frame focused=201`

## Root Cause
Focused surface `201` (Quil) was present and focused, but the keyboard proof/action path resolves frame ownership via `frame_for_surface(surface_id)`, which scans `FRAMES[*].tabs[*].surface_id`.

Boot frame/tab model did not reliably include the focused app surfaces (`201` Quil, `200` Linen) in `FRAMES` tabs at proof time, so `frame_for_surface(201)` returned `None` and proof skipped with `no_frame`.

## Changes
File changed:
- `servers/silk-shell/src/main.rs`

### 1) Added focused-surface -> frame lookup diagnostics in keyboard proof path
Inside `maybe_run_keyboard_window_synthetic_proof()`:

- Added:
  - `[shell.focus.frame.lookup] focused=N frame=N ok=N reason=...`
- On no-frame, added bounded current frame-tab map dump:
  - `[shell.frame.surface.map] frame=N sid=N kind=active_tab active=1`
- Preserved skip marker:
  - `[shell.keyboard.window.proof.skip] reason=no_frame focused=N`

### 2) Ensured app surfaces are attached to frame model at boot
At boot init path, added local frame-map attach calls and markers:

- `ensure_quil_frame()` with marker:
  - `[shell.frame.surface.map] frame=<fid> sid=201 kind=boot_attach active=1`
- `ensure_linen_frame()` with marker:
  - `[shell.frame.surface.map] frame=<fid> sid=200 kind=boot_attach active=1`

This keeps focus model unchanged and only guarantees owning frame resolution for existing app surfaces.

## Behavior Scope
- No kernel/ABI/opcode changes.
- No sexdisplay changes.
- No focus-policy redesign.
- No destructive proof actions.

## Build
Command:
- `SEXOS_KEYBOARD_WINDOW_PROOF=1 ./scripts/entrypoint_build.sh`

Result:
- Build success (`[SEXOS ENTRYPOINT] success` observed).

## Runtime Proof Command
```bash
grep -E "shell.keyboard.window.proof|shell.focus.frame.lookup|shell.frame.surface.map|shell.key.action|shell.window.action|shell.frame.zoom|shell.frame.unzoom|shell.frame.minimize|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | tail -1200
```

## Expected Proof Outcome
- `reason=no_frame focused=201` should disappear once frame attach is present.
- Proof trigger/stage markers should execute after focus/frame readiness.
- Window action path should resolve frame from focused surface.
- Fault count remains 0.
