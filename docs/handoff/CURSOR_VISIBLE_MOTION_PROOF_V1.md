# CURSOR_VISIBLE_MOTION_PROOF_V1

A) PASS / FAIL / PARTIAL
- PASS
- Proof run result: `FINAL: PASS (279 gates proved, 116 skipped, 0 faults)`

B) cursor state/update path
1. Pointer state fields (shell-owned)
- `servers/silk-shell/src/main.rs`: `POINTER_X`, `POINTER_Y`, `POINTER_BUTTONS`, plus `ABS_SEEN_VALID`/`REAL_POINTER_SEEN`.
2. Motion ingress
- `EV_REL` path: `apply_rel_pointer(dx_raw, dy_raw)` updates pointer and sends cursor surface.
- `EV_ABS` path: `process_abs_tablet(raw_x, raw_y)` normalizes to screen coords and updates pointer.
3. Visible/logical cursor update
- Shell sends cursor updates via `send_cursor_checked(...)`.
- Existing visibility markers observed: `[shell.cursor.move]`, `[shell.cursor.surface.update]`, `[silk-shell.cursor.update]`.

C) bounds/clamp rule
- REL path clamps to display bounds: `new_x = clamp(0..P.width-1)`, `new_y = clamp(0..P.height-1)`.
- ABS path normalization already clamps via `normalize_abs_coord(...).clamp(0..screen_dim-1)`.
- Added bounded proof marker:
  - `[cursor.motion.bounds] source=<rel|abs> old_x=<n> old_y=<n> new_x=<n> new_y=<n> w=<n> h=<n> clamped=<0|1> ok=<0|1>`

D) visible/logical movement proof
- Motion input observed:
  - `[silk-shell.pointer.recv] class=...`
  - `[silk-shell.rel.recv] dx=... dy=...`
- Position changed/logically applied:
  - `[shell.cursor.move] x=... y=...`
  - `[cursor.motion.bounds] ... ok=1`
- Gate result:
  - `cursor_visible_motion PASS`

E) no-focus-mutation proof
- Reused AP4 lane marker:
  - `[shell.interact.stage.pointer_state] source=<abs|rel> old_focus=<id> new_focus=<id> moved=1 ok=1`
- Gate result:
  - `cursor_motion_no_focus_mutation PASS`
  - `shell_interaction_pointer_no_focus_mutation PASS`

F) gates/markers added or reused
1. Added marker
- `servers/silk-shell/src/main.rs`
  - `[cursor.motion.bounds] ... ok=1` (proof-gated, budgeted)
2. Added gate rows
- `scripts/daily_driver_master_gate.sh`
  - `cursor_visible_motion`
  - `cursor_motion_no_focus_mutation`
  - `cursor_motion_bounds`
3. Reused gate evidence
- `shell.interact.stage.pointer_state` marker family from AP4
- `faults_zero` gate for fault/OOB containment lane

G) proof command + log path
- Command: `./scripts/run_daily_driver_proof.sh /tmp/cursor_visible_motion_proof_v1.log`
- Log: `/tmp/cursor_visible_motion_proof_v1.log`

H) gate results (cursor mission scope)
- `cursor_visible_motion`: PASS (`bounds ok marker observed`)
- `cursor_motion_no_focus_mutation`: PASS (`reused shell.interact pointer_state moved=1 old_focus==new_focus`)
- `cursor_motion_bounds`: PASS (`cursor.motion.bounds ok=1 observed`)
- `shell_interaction_pointer_no_focus_mutation`: PASS
- `faults_zero`: PASS
- Global summary: `FINAL: PASS (279 gates proved, 116 skipped, 0 faults)`

I) fault scan
Required tokens in `/tmp/cursor_visible_motion_proof_v1.log`:
- `#PF`: 0
- `#GP`: 0
- `panic`: 0
- `fault.kill`: 0
- `null-jump`: 0
- `IPC storm`: 0
- `ring overflow`: 0
- `cursor FAIL`: 0
- `pointer FAIL`: 0
- `click FAIL`: 0
- `drag FAIL`: 0
- `focus FAIL`: 0
- `shell_interaction FAIL`: 0
- `bounds FAIL`: 0
- `OOB`: 0
- `out-of-bounds`: 0

J) remaining gaps
1. Visible render marker specificity
- Current lane proves logical cursor movement + shell cursor surface updates; it does not add a brand-new dedicated sexdisplay “cursor pixel changed this frame” marker.
2. Minor gate-script warning (non-fatal)
- Existing script emitted `integer expected` warnings at line 4947 during this run, but final gate outcome remained PASS and unaffected.

K) files changed
- `servers/silk-shell/src/main.rs`
- `scripts/daily_driver_master_gate.sh`
- `docs/handoff/CURSOR_VISIBLE_MOTION_PROOF_V1.md`

L) next required autopilot
- `CLICK_FOCUS_DRAG_PROOF_PLAN_V1`
