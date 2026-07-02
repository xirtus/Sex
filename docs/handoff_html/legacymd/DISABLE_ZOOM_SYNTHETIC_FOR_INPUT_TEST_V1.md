# DISABLE_ZOOM_SYNTHETIC_FOR_INPUT_TEST_V1

## 1. Scope

Disable only the synthetic frame-light zoom proof during normal boot/input testing.

- No input routing changes
- No frame-light behavior changes
- No zoom/unzoom action changes
- No kernel, ABI, sexdisplay, renderer, or opcode changes

## 2. Change

File changed:

- `servers/silk-shell/src/main.rs`

Added a local gate constant with default OFF:

- `const ENABLE_FRAME_LIGHT_ZOOM_SYNTHETIC_PROOF: bool = false;`

`maybe_run_frame_light_zoom_synthetic_proof()` now exits early when disabled and emits at most one marker:

- `[frame.light.zoom.synthetic.skip] reason=disabled`

## 3. Why

Input milestone proofing (USB tablet/keyboard/PS2/click-drag lifecycle) should run without synthetic GUI zoom noise in the default lane.

## 4. Re-enable

For a dedicated synthetic zoom proof session, toggle the local constant to `true` in `servers/silk-shell/src/main.rs`.

## 5. Verification

- Build gate: `./scripts/entrypoint_build.sh`
- Runtime expectation in default builds:
  - One skip marker appears
  - No synthetic zoom trigger/begin/click/done markers appear
