FRAME_LIGHT_ZOOM_SYNTHETIC_PROOF_V1

Purpose
-------
Prove the green (zoom) frame-light can be activated using the exact same
hit-test → action path a real pointer click uses.

Summary
-------
A new helper in servers/silk-shell/src/main.rs (unsafe fn
synthetic_prove_frame_light_zoom_click(frame_id: u32) -> bool) computes a
candidate click coordinate inside the frame's top-left light band and calls
click_hit_test_and_focus(px, py, 1). That function runs the same hit-test
and frame-light action logic (close/minimize/zoom) used by real clicks.

How it computes the target
-------------------------
- Resolves the frame's active surface bounds (sx, sy).
- Probes x offsets in [sx, sx+80) stepping by 2px at y = sy + (top_bar_h/2),
  calling frame_light_at(frame_id, x, y) until FRAME_LIGHT_ZOOM is found.
- Falls back to sx+50 if probe fails.

Why this is a valid "proof"
----------------------------
- The synthetic click calls click_hit_test_and_focus(px,py,1) — the *same*
  function called for real pointer-down events, so the chrome hit-test,
  light detection, and action dispatch are exercised identically.
- The helper logs diagnostic markers: [shell.proof.zoom.click] and
  [shell.proof.zoom.result] and reuses existing budgeted diagnostics.

Running and verification
------------------------
1. Build: ./scripts/entrypoint_build.sh (run in the normal build environment).
2. The synthetic proof is run one-shot at boot (QUIL frame) and logs to serial.
3. Expected runtime markers (grep for these in the serial log):
   - [shell.frame.chrome.bounds]             (frame chrome bounds print)
   - [frame.light.zoom.synthetic.begin]      (begin marker with px/py)
   - [shell.frame.light.hitbox]              (produced by frame_light_at during probe)
   - [shell.hit_target.chrome]               (hit-target classified as chrome)
   - [frame.light.zoom.fsm]                  (zoom FSM diagnostic during toggle)
   - [shell.frame.zoom] or [shell.frame.unzoom] (actual zoom/unzoom action)
   - [frame.light.zoom.synthetic.done]       (final success marker ok=1)

Runtime grep (example):
grep -E "frame.light.zoom.synthetic|shell.frame.light.hitbox|shell.hit_target.chrome|frame.light.zoom.fsm|shell.frame.(zoom|unzoom)|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | head -260

Build result placeholder
------------------------
- Fill in after build: (PASS/FAIL) and paste key grep output lines.

Notes & Safety
--------------
- No kernel, sex-pdx, sexdisplay, or renderer changes were made.
- The helper is unsafe (matches surrounding code style) and only invokes
  internal shell helpers; it does not change ABIs.
- Backup of main.rs created before edit: servers/silk-shell/src/main.rs.bak-20260509T052144

If something goes wrong
----------------------
- STOP FIRST: read docs/handoff and claude-references/*.md before further edits.
- Restore the backup file if needed.
