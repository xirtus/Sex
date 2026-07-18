# SILK_WINDOW_MOVE_TEXT_INPUT_CURRENT_TIER_V1

## Mission

Click focus -> drag move -> focused typing, current tier. Shell-only patch.

## Result — PASS (2026-07-05)

- Build: `./scripts/entrypoint_build.sh` PASS
- Runtime: `GATE_DIR=/tmp/silk_move_text_z1 PROBE_SECONDS=18 POST_STIMULUS_TIMEOUT_SECONDS=24 ./scripts/gate_0_2.sh`
- Chapter 1: `scripts/input_current_tier_gate.sh logs/qemu-latest.log` -> `INPUT_100_CURRENT_TIER_V1: PASS`
  (all 14 legacy rows + 13 new mission rows)
- Chapter 2: `scripts/input_control_quality_gate.sh logs/qemu-latest.log` -> `INPUT_PRESENT_TICK_TRACE_V1: PASS`
- Faultscan on passing log: pf=0 gp=0 panic=0 fault_kill=0 reboot_loop=0 freeze=0 storm=0
- Passing log preserved at /tmp/passing_lane.log for this session.
- Typing proven via real QMP PS/2 keys (`key 1 h i spc h backspace`, then `key 1 slash`
  negative): `[silk.text.append] ch=H,I,<space>,H`, `[silk.text.backspace] len=3`.
- Frame move proven: `[silk.window.move.proof.result] sid=201 from=(0,50) to=(18,50) moved=1`
  (one-shot rim-drag proof) plus real QMP drag on surface 100 in the same run
  (`[silk.window.move.proof.done] ok=1`).

## What was missing (Phase A audit)

1. Pointer state: silk-shell (`POINTER_X/Y/BUTTONS`). Keyboard: silk-shell,
   two EV_KEY dispatch sites (handle_hid_event drain ~9500; main OP_HID_EVENT
   dispatch ~23900). sexinput stays producer-only; sexdisplay pixels-only.
2. `drag_move_focused` moved only legacy windows 100-103. Frame app surfaces
   (Linen 200, Quil 201, Mesh 202, Collar 203, Bell 204) could BEGIN a rim
   drag but never moved — the compositor 0xEB/0xEC paths were fine; the gap
   was shell-local geometry mutation. Same gap in the arrow-key move block.
3. No shell-owned text sink. Quil has a full text buffer, but it is a
   separate PD (existing app text protocol — left untouched).
4. Browser stub (sid 205) is registered focusable but missing from
   `surface_is_alive` -> permanently unfocusable. Used as the deterministic
   `invalid_surface` negative-proof target.

## Implementation (silk-shell only, no new ABI)

- `move_surface_by(sid, dx, dy) -> Option<(x, y, clamped)>`: one clamped move
  helper for 100-103 AND 200-204. `emit_snapshot()` already pushes 200-204
  positions via OP_SURFACE_UPDATE (0xEB), so frames move with zero
  compositor changes.
- `drag_move_focused` now uses it -> rim-drag moves frames; emits
  `[silk.drag.move] surface= x= y= clamped=`.
- Arrow-key move block extended to Quil/Mesh/Collar/Bell via the same helper
  (`[silk.window.nudge]`). Linen arrows already existed.
- Focused text sink: surface 100 (`SILK_TEXT_SINK_SID`), 96-byte buffer.
  Non-reserved printables map to UPPERCASE ASCII (`sink_scancode_to_char`) —
  sexdisplay's 5x7 renderer drops bytes > 0x5A, lowercase would be invisible.
  Backspace (0x0E) is sink-owned while 100 is focused (both key edges
  consumed); AccessFocusPrev works from any other focus. Redraw per keystroke:
  OP_TEXT_CLEAR (0xFA) + shell_draw_text (0xFB), bounded <= 13 display calls.
  Wired into BOTH EV_KEY dispatch sites, before reserved-UI consumption.
- Mission markers added at existing sites (focus click, drag begin/end via
  shared `silk_drag_begin_mark`/`silk_drag_end_mark`, key route/reject,
  focus reject, drag reject).
- Negative proofs (env-gated, lane-enabled, non-interfering):
  - `SEXOS_SILK_FOCUS_REJECT_PROOF`: one-shot `try_set_focus(205)` (dead) ->
    `[silk.focus.reject] reason=invalid_surface`; restores nothing (focus
    unchanged by construction).
  - `SEXOS_SILK_DRAG_REJECT_PROOF`: one-shot click at (1270,700) through
    handle_hid_event -> no drag begins, `drag_moves_delta=0`; restores
    pre-proof focus (a click focuses whatever tile is under it — the boot
    layout is fully tiled, there are NO background pixels).
  - `SEXOS_SILK_WINDOW_MOVE_PROOF` (new, one-shot): full rim-drag sequence
    (EV_ABS + BTN down + EV_REL + BTN up) in a single call — the staged
    `SEXOS_WINDOW_DRAG_PROOF` starves because the main loop iterates per
    message and the Enter autopilot minimizes Quil before its stage 1 runs.
    Target selection (`synthetic_window_drag_target`) got a candidate
    fallback (Quil -> Linen -> Mesh -> Collar -> focused) + reject
    diagnostics for the same reason.
- Drain-path parity fix: digit '1' (`SurfaceAction::Focus100`) now dispatches
  `try_set_focus(100)` in the handle_hid_event drain path (main dispatch
  already did) — the lane uses it to pin focus before typing, since a stray
  pointer press can click-focus an app tile.

## Files changed

- `servers/silk-shell/src/main.rs` (backup: `main.rs.bak.silk_move_text_v1`)
- `scripts/gate_0_2.sh` (backup: `.bak.silk_move_text_v1`): typing stimulus
  (`qmp_input_probe.py key h i spc h backspace`), `slash` key-reject negative,
  proof env exports, paced left sweep + one verified retry.
- `scripts/input_current_tier_gate.sh` (backup: `.bak.silk_move_text_v1`):
  13 new marker rows (positive + negative).

## Proof markers

Positive: `[silk.focus.click]`, `[silk.drag.begin]`, `[silk.drag.move] ... clamped=`,
`[silk.drag.end]`, `[silk.window.move.proof.done] ok=1`, `[silk.key.route]`,
`[silk.text.append]`, `[silk.text.backspace]`, `[silk.text.draw]`,
`[silk.text.input.proof.done] ok=1`, `[silk.window.nudge]`.
Negative: `[silk.drag.reject] reason=no_surface ok=1`,
`[silk.key.reject] reason=no_focus ok=1`,
`[silk.focus.reject] reason=invalid_surface ok=1`.

## Hard-won lane facts (read before touching gate_0_2.sh again)

1. **sexinput smoothing makes QMP pointer sweeps crawl**: each processed
   tablet report becomes a ~1-3px bounded step (`[usb.tablet.delta.clamp]`).
   Host sweeps cannot deterministically reach a distant screen region; when
   the host floods, the xHCI ring coalesces reports and the pointer
   undershoots further. Pointer-position-dependent QMP stimulus is inherently
   flaky. Use shell-side synthetic proofs (real handle_hid_event path) for
   position-critical interactions.
2. **The boot layout is fully tiled** (100 left / Quil mid / Linen right,
   sizes from boot tiling, NOT the static defaults — Linen tile covers the
   whole right column). There is no background to click.
3. **Enter (AccessActivate) runs a GUI autopilot** that minimizes the Quil
   frame and lands focus on surface 100 — that is why the lane's `ret`
   injection produces `focus.set id=100`. Anything expecting the Quil frame
   alive must run before Enter or pick another frame.
4. **The shell main loop iterates per received message**, so `maybe_run_*`
   proof stage machines advance slowly and can be starved until first input.
5. **Budgeted markers** (`usb.pointer.shell.apply` ~64, click/drag budgets)
   cannot be used to verify late-run pointer state.
6. **Kernel PF flake**: `Scheduler::tick` deref at RIP 0xffffffff80220a38
   (addr 0x58/0x68, pd=8) killed ~60-70% of lane runs on 2026-07-05, usually
   during idle/clock or early stimulus. Pre-existing (see memory/BUG notes:
   pd_ptr symbolization). NOT touched (kernel = STOP FIRST). Retry the lane.

## Remaining current-tier limitations

- Text sink targets surface 100 only; reserved-UI scancodes (digits 1-5, J,
  K, L, M, C, R, -, =, [, ], ;, backtick, Tab/Esc/Enter/F-keys) are not
  typeable — they keep their shell action semantics.
- Frame surfaces 205 (browser stub) and Spindle are not draggable (not in
  emit_snapshot's 0xEB push set). Browser is not alive-listed at all.
- QMP-driven real-pointer drag onto surface 100 works only when the sweep
  lands (see lane fact 1); the deterministic drag proof covers the gate.
- Kernel scheduler flake above.
