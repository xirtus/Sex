# SHELL_INTERACTION_STATE_CONTRACT_V1

A) PASS / FAIL / PARTIAL
- PASS (contract defined, mapped to current code, regression proof PASS)
- No runtime behavior patch applied in AP3 (docs-only output)

B) Current state owner map
1. `silk-shell` (owner of interaction policy/state)
- Pointer state: `POINTER_X`, `POINTER_Y`, `POINTER_BUTTONS`, wheel/rel accumulators
- Interaction state machine: `InteractionState::{Idle,ClickPending,Dragging,PanelActive,Resizing,TabDragging}`
- Focus/keyboard target: `FOCUSED_SURFACE_ID` + `access_handle_keyboard_action(...)`
- Hit-testing, click focus, drag/resize decisions: `click_hit_test_and_focus`, `drag_move_focused`, `resize_accumulate_delta`, `apply_resize_geometry`
- Cleanup/liveness guards: `clear_focus_if_dead`, `clear_drag_if_dead`, `clear_hover_if_dead`, `surface_is_alive`, tombstone checks
- Shortcut consumption: `scancode_to_action` + `shell.kbd.ui.consume/result` path before app routing
2. `sexdisplay` (owner of rendering)
- Surface compositing, focused draw precedence, chrome pixel rendering, framebuffer writes
- No primary ownership of input-policy routing
3. `sexinput` (owner of input production)
- PS/2/USB intake and normalization into `OP_HID_EVENT` (`EV_KEY`, `EV_REL`, `EV_ABS`, `EV_BTN`)
- Sends to shell slot; no focus policy / no app policy routing decisions

C) Proposed canonical interaction state contract
1. Ownership contract
- `silk-shell` owns interaction policy/state transitions and event routing decisions.
- `sexdisplay` owns rendering execution only.
- `sexinput` owns event production/normalization only.
2. Route boundary contract
- Keyboard route remains shell-centric (`... -> sexinput -> silk-shell -> shell consume or focused app`).
- Pointer route remains shell-centric for capture/focus/drag/resize policy.
- No keyboard reroute through sexdisplay.
3. Safety boundary contract
- No cross-PD raw pointer contracts for interaction.
- Liveness/capability checks must occur in shell before routing mutation and before delivery to app targets.

D) Current implementation mapping (present / partial / missing)
1. Present
- Interaction owner in shell: explicit state machine and pointer/focus globals.
- Shell-first shortcut consume before app route (`shell.kbd.ui.consume` then return).
- Focus/drag/hover dead-target cleanup paths present and exercised (`skip_dead`, clear_dead markers).
- Button-up clears active interaction modes via transition to `Idle` for click/drag/resize/tab-drag.
- Click-driven focus mutation path present (`click_hit_test_and_focus` markers).
- Route-to-focused-app path present for Quil/Linen/Spindle (and focused mesh path).
2. Partial
- Priority ordering exists implicitly in control flow, but not yet published as a strict canonical contract section in code/docs.
- Capture precedence for overlays/chrome/atlas is implemented, but some branches are spread across main loop + helpers (not centralized table-driven policy).
- Shortcut set is robust for existing actions, but explicit formal proof markers for specific modifier combos (Alt/Shift/Ctrl matrix) are incomplete.
3. Missing / ambiguous for AP4
- Explicit contract-level marker family for each ordering stage (normalize/capture/consume/route/commit) is incomplete.
- Dedicated marker proving “cursor movement alone never mutates focus” as a strict invariant is not yet isolated as its own proof row.
- Dedicated marker proving “shell declines shortcut then app receives key” as explicit two-step evidence for same event ID is not yet fully stitched.

E) Event ordering contract (canonical)
1. Input normalization (`sexinput`) into typed HID-like events
2. Shell raw state update (`POINTER_*`, keyboard edge bookkeeping)
3. Liveness cleanup (`clear_*_if_dead`, tombstone/lifecycle checks)
4. Capture/priority resolve (modal/active drag-resize/chrome/atlas/etc.)
5. Shell-reserved shortcut and click-zone consume path
6. Focus/drag/resize state mutation (if action/hit-test allows)
7. Route remaining key/pointer event to focused app target
8. Rendering commit remains separate (`sexdisplay`)

F) Priority/capture order (highest -> lowest)
1. Fatal/liveness cleanup (dead/tombstoned/unfocusable target guards)
2. Secure/system modal (if active)
3. Active drag/resize capture
4. Top strip/SilkBar/shell chrome hit regions
5. Atlas/overlay mode capture
6. Focused frame/app surface routing
7. Background/workspace fallback

G) Invariants
1. Dead target never receives keyboard/pointer routing.
2. Focus target must be live + lifecycle-focusable.
3. Drag/resize target must be live on every move tick.
4. Button-up exits drag/resize/click-pending capture states.
5. Shell shortcut consume must prevent duplicate leak to app route.
6. App key delivery occurs only after shell declines shortcut.
7. Pointer move alone cannot mutate focus.
8. Focus mutation via pointer requires click hit-test path.
9. No renderer-owned policy decisions.
10. No framebuffer OOB writes.
11. No IPC storm / panic / `#PF` / `#GP` / `fault.kill` in proof lane.

H) Missing AP4 implementation items (smallest useful scope)
1. Add canonical marker family for ordering stages (receive -> consume/decline -> route -> post-state).
2. Add explicit marker proving “shortcut declined => app route selected” for same dispatch lane.
3. Add explicit marker proving “EV_REL/EV_ABS updates cursor without focus mutation” in same window.
4. Add dedicated contract gate rows for interaction-state invariants (not broad subsystem gates).
5. Add focused negative-case marker: no-focus target path consumes/ignores safely without app send.

I) Missing proof markers needed for AP4
1. `shell.interaction.contract.begin` / `.done`
2. `shell.interaction.stage.normalize.ok`
3. `shell.interaction.stage.capture.ok`
4. `shell.interaction.stage.shortcut.consume|decline`
5. `shell.interaction.stage.route.target=<sid|none> ok=1`
6. `shell.interaction.invariant.cursor_no_focus_mutation ok=1`
7. `shell.interaction.invariant.dead_target_blocked ok=1`
8. `shell.interaction.invariant.shortcut_no_leak ok=1`
9. `shell.interaction.invariant.button_up_clears_capture ok=1`

J) Proof command + log path
- Command: `./scripts/run_daily_driver_proof.sh /tmp/shell_interaction_state_contract_v1.log`
- Log: `/tmp/shell_interaction_state_contract_v1.log`
- Result: `FINAL: PASS (272 gates proved, 115 skipped, 0 faults)`

K) Fault scan (required tokens)
- `#PF`: 0
- `#GP`: 0
- `panic`: 0
- `fault.kill`: 0
- `null-jump`: 0
- `IPC storm`: 0
- `ring overflow`: 0
- `keyboard FAIL`: 0
- `cursor FAIL`: 0
- `pointer FAIL`: 0
- `click FAIL`: 0
- `drag FAIL`: 0
- `focus FAIL`: 0

L) Files changed
- `docs/handoff/SHELL_INTERACTION_STATE_CONTRACT_V1.md` (new)

M) Next required autopilot
- `SHELL_INTERACTION_STATE_IMPL_V1`

## Note
- `docs/handoff/AGENT_HANDOFF_GP_CLOCK.md` is missing in this checkout.
