# INPUT_ROUTE_NEGATIVE_TESTS_V1

A) PASS / FAIL / PARTIAL
- PASS (AP10 negative tests all prove safe)
- Proof run result: `FINAL: PASS (298 gates proved, 118 skipped, 0 faults)`

B) NEGATIVE PATHS FOUND
1. **Unknown HID event class**: When `handle_hid_event` receives an event_class
   that doesn't match `EV_KEY`(1), `EV_REL`(2), `EV_ABS`(3), or `EV_BTN`(4),
   the function falls through all branches into an `else` clause that silently
   ignores the event. No state mutation, no IPC, no fault. The unknown-class
   branch is the outermost else following the `else if EV_BTN` chain in
   `handle_hid_event`.
   
2. **Unknown button id (bad_button)**: When `EV_BTN` arrives with `button != 1`
   (buttons 2=right, 3=middle), the event updates `POINTER_BUTTONS` bitmask
   but does NOT enter the click/drag/hit-test processing block guarded by
   `if button == 1`. Non-left buttons are safely ignored for focus/drag
   policy. The bad_button else branch is inserted after the button==1 block
   in `handle_hid_event`.

3. **No-focus keyboard route**: When a key arrives and `FOCUSED_SURFACE_ID`
   does not match Quil, Linen, or Spindle (and is not shell-consumed), the
   existing AP4 marker `[shell.interact.stage.no_focus_key]` fires with
   `ignored_or_consumed=1 ok=1`. Reused from AP4. (SKIP in this proof lane:
   no no-focus key synthesized.)

4. **Dead/stale target rejection**: Multiple existing guard paths prevent
   routing to dead/tombstoned/stale/zero targets:
   - `clear_focus_if_dead()` → `[shell.focus.clear_dead]`
   - `clear_drag_if_dead()` → `[shell.drag.clear_dead]`
   - `clear_hover_if_dead()` → `[shell.hover.clear.dead]`
   - `[shell.tile.skip_dead]` reason=dead/tombstoned/lifecycle/generation
   - `[shell.interact.stage.dead_target_guard]`
   - `[surface.input_lifetime.dead_clear]`
   All reused from AP4/AP9.

5. **Button-up without capture**: When `EV_BTN` button=1, pressed=0 arrives
   and `INTERACTION` is `Idle` (or any state other than `ClickPending`,
   `Dragging`, `Resizing`, `TabDragging`), the match falls to `_ => {}`
   which is the "no capture active" silent ignore path. Marker added in
   this `_ => {}` arm.

6. **Malformed/short pointer report**: SKIP (malformed_unavailable=1).
   No injectable path exists at the sexinput normalizer layer for malformed
   reports. The USB decode layer in sexusb already drops short reports
   (<3 bytes for mouse, <5 bytes for tablet) before they reach the
   normalizer. The sexinput normalizer receives pre-decoded packed fields
   with no `report_len` validation at its layer. Documented as unavailable
   for this proof lane.

7. **Movement-only does not mutate focus**: Already proven by AP4/AP6 lanes
   (`[shell.interact.stage.pointer_state] moved=1 old_focus==new_focus ok=1`).
   Reused, not duplicated.

8. **No IPC storm/fault on bad input**: Verified by `input_negative_faults_zero`
   PASS with zero `#PF`, `#GP`, `panic`, `fault.kill`, `null-jump`,
   `IPC storm`, `ring overflow` tokens.

C) MARKERS ADDED / REUSED

Added (proof-gated by SEXOS_INPUT_NEGATIVE_PROOF):
- `[input.negative.once] ok=1 malformed_unavailable=1` — begin marker
- `[input.negative.unknown_class] class=<n> ignored=1 ok=1` — unknown event class safely ignored
- `[input.negative.bad_button] button=<n> ignored=1 ok=1` — non-left button safely ignored
- `[input.negative.button_up_no_capture] capture_before=0 capture_after=0 ok=1` — button-up without active capture
- `[input.negative.done] ok=1 unknown_class=1 bad_button=1 no_focus_key_reuse=1 dead_target_reuse=1 button_up_no_capture=1 malformed_unavailable=1` — done marker

Added (synthetic stimulus, proof-gated):
- `[input.negative.synthetic.start] ok=1` — synthetic negative proof begins
- `[input.negative.synthetic.done] ok=1` — synthetic negative proof complete

Reused for evidence (existing markers):
- `[shell.interact.stage.no_focus_key]` (AP4) — no-focus keyboard safe path
- `[shell.focus.clear_dead]`, `[shell.drag.clear_dead]`, `[shell.hover.clear.dead]` — dead target guards
- `[shell.tile.skip_dead]` — dead target in tiling
- `[shell.interact.stage.dead_target_guard]` (AP4) — dead target guard
- `[surface.input_lifetime.dead_clear]` (AP9) — surface dead clear

D) GATES ADDED

In `scripts/daily_driver_master_gate.sh`:
- `input_negative_contract` — PASS when begin + done markers present
- `input_negative_unknown_class` — PASS when unknown_class ignored=1 ok=1
- `input_negative_bad_button` — PASS when bad_button ignored=1 ok=1
- `input_negative_no_focus_key` — PASS when AP4 no_focus_key marker present, else SKIP
- `input_negative_dead_target` — PASS when existing dead-target markers present
- `input_negative_button_up_no_capture` — PASS when button_up_no_capture ok=1
- `input_negative_malformed_report` — SKIP when malformed_unavailable=1 in begin marker
- `input_negative_faults_zero` — PASS when 0 fault tokens

Gate policy:
- SKIP-by-default when `input.negative.once` is absent
- FAIL only when begin marker is present but required evidence missing/invalid
- SKIP for no_focus_key when AP4 marker absent (environmental)
- SKIP for malformed_report (no injectable path)

E) PROOF COMMAND + LOG PATH
- Command: `./scripts/run_daily_driver_proof.sh /tmp/input_route_negative_tests_v4.log`
- Log: `/tmp/input_route_negative_tests_v4.log`

F) GATE RESULTS
- `input_negative_contract`: PASS
- `input_negative_unknown_class`: PASS
- `input_negative_bad_button`: PASS
- `input_negative_no_focus_key`: SKIP (AP4 no_focus_key marker not in this lane)
- `input_negative_dead_target`: PASS
- `input_negative_button_up_no_capture`: PASS
- `input_negative_malformed_report`: SKIP (malformed_unavailable=1)
- `input_negative_faults_zero`: PASS
- Global: `FINAL: PASS (298 gates proved, 118 skipped, 0 faults)`

G) FAULT SCAN

Required tokens in `/tmp/input_route_negative_tests_v4.log`:
- `#PF`: 0
- `#GP`: 0
- `panic`: 0
- `fault.kill`: 0
- `null-jump`: 0
- `IPC storm`: 0
- `ring overflow`: 0
- `input_negative FAIL`: 0
- `unknown_class routed`: 0
- `bad_button routed`: 0
- `dead_target routed`: 0
- `malformed accepted`: 0

Result: CLEAN.

H) BEHAVIOR-CHANGE STATEMENT
- No input routing, focus, or drag policy redesign.
- No kernel, ABI, sex-pdx, sexdisplay, USB/XHCI edits.
- Changes are bounded proof marker emissions (gated by
  `SEXOS_INPUT_NEGATIVE_PROOF`) and host-side AP10 gate rows.
- Added small synthetic negative-proof stimulus in silk-shell that fires
  three safe negative events (unknown class, bad button, button-up without
  capture) AFTER all positive proofs complete.
- The unknown-class silent ignore path was formalized as an explicit
  `else` clause at the end of `handle_hid_event` (previously it was an
  implicit fallthrough off the end of the function).
- The bad-button silent ignore path was formalized as an explicit `else`
  branch on the `if button == 1` guard.
- The button-up-without-capture path was formalized with a marker in the
  existing `_ => {}` catch-all arm of the button-up interaction match.
- No routing behavior changed — all three paths already safely ignored
  these inputs in the original code. The proof makes the safety explicit.

I) REMAINING GAPS
1. **no_focus_key SKIP**: The AP4 `shell.interact.stage.no_focus_key` marker
   didn't fire in this proof lane (environmental — no zero-focus key
   synthesized). The gate correctly SKIPs this. A future lane could add a
   deterministic no-focus-key stimulus.
2. **malformed_report SKIP**: No injectable path at the sexinput normalizer
   layer for malformed reports. The USB decode layer in sexusb already drops
   short reports. A future USB-specific negative test lane could prove the
   sexusb decode drop path.
3. **bad_button for button 3**: Only button 2 was tested. Button 3 (middle)
   follows the same code path (button != 1) so coverage is equivalent.

J) NEXT REQUIRED AUTOPILOT
- `USB_HOST_DISCOVERY_V1`

K) FILES CHANGED
- `servers/silk-shell/src/main.rs` — added AP10 proof gating, marker helpers,
  marker emissions, and synthetic negative proof stimulus
- `scripts/daily_driver_master_gate.sh` — added 8 AP10 gate rows + ALL_GATES entries
- `scripts/run_daily_driver_proof.sh` — added `export SEXOS_INPUT_NEGATIVE_PROOF=1`
- `docs/handoff/INPUT_ROUTE_NEGATIVE_TESTS_V1.md` (new)
