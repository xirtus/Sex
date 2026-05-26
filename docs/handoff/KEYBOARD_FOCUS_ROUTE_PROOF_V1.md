# KEYBOARD_FOCUS_ROUTE_PROOF_V1

A) PASS / FAIL / PARTIAL
- PASS (current-tier keyboard focus route proof)
- Proof run result: `FINAL: PASS (272 gates proved, 115 skipped, 0 faults)`

B) Exact keyboard route proven
1. IRQ/scancode source
- `kernel/src/interrupts.rs`: IRQ1 (`0x21`) reads scancode and enqueues into `INPUT_RING` (`[ps2.input_ring.enqueue]`).
2. sexinput EV_KEY construction
- `servers/sexinput/src/main.rs`: `pdx_try_listen_raw(SLOT_INPUT)` consumes type `0x201`, strips break bit, emits `pdx_call(SLOT_SHELL, OP_HID_EVENT, code, value, EV_KEY)`.
3. silk-shell receive + policy
- `servers/silk-shell/src/main.rs`: `handle_hid_event(EV_KEY, ...)` logs `[silk-shell.key.recv]`, consumes shell-owned UI actions first (`[shell.kbd.ui.consume]`, `[shell.kbd.ui.result]`), then routes to focused app slot.
4. focused app route
- Linen route observed: `[silk-shell.key.route] target=linen sid=200 ...`
- Mesh route observed: `[silk-shell.key.route] target=mesh sid=202 ...`
- Quil physical path setup observed (`[physical_keyboard.quil.begin]`), but completion remained SKIP (environmental).

C) Proof command(s)
- `./scripts/run_daily_driver_proof.sh /tmp/keyboard_focus_route_proof_v1.log`
- Gate scanner (invoked by runner): `./scripts/daily_driver_master_gate.sh /tmp/keyboard_focus_route_proof_v1.log`

D) Proof log path(s)
- `/tmp/keyboard_focus_route_proof_v1.log`

E) Markers found
- Keyboard ingress:
  - `[ps2.irq1.entry]`
  - `[sexinput.ps2.scancode]`
  - `[silk-shell.key.recv]`
- Focus-policy and recipient switching:
  - `[shell.kbd.ui.consume]`
  - `[shell.kbd.ui.focus] old=... new=... reason=...`
  - `[shell.kbd.ui.result]`
  - `[silk-shell.key.route] target=linen ...`
  - `[silk-shell.key.route] target=mesh ...`
- Keydown/keyup evidence:
  - down events: e.g. `code=67 down=1`
  - up events: e.g. `code=67 down=0`

F) Gates passed/skipped (keyboard-focus relevant)
- `quil_keyboard` PASS
- `scene_keyboard_switch` PASS
- `keyboard_gui` SKIP (`not_requested`, explicit begin marker missing)
- `physical_keyboard_to_quil_text` SKIP (setup present, done marker absent; environmental limitation path)
- `faults_zero` PASS

G) Skipped proof reason if any
1. `keyboard_gui`
- Gate requires explicit begin marker (`keyboard.gui.proof.begin` / equivalent).
- Current run did not emit begin sentinel, so honest SKIP by gate policy.
2. `physical_keyboard_to_quil_text`
- `[physical_keyboard.quil.begin]` present, but no `[physical_keyboard.quil.done]`; runner appends v2 skip semantics for environmental QEMU sendkey/boot-window limitations.

H) Negative test result
- Dead/unfocus-safe behavior evidence present:
  - Dead-surface exclusion markers repeatedly observed: `[shell.tile.skip_dead] ... reason=dead`
  - Focus clear safety observed: `[tiling.focus.clear] ... reason=invalid_after_tiling`
  - No-focus keyboard safety observed: `[shell.kbd.ui.action] ... focused=0 frame=0 sid=0` with no crash/fault markers.

I) Fault scan result
Required scan tokens in `/tmp/keyboard_focus_route_proof_v1.log`:
- `#PF`: 0
- `#GP`: 0
- `panic`: 0
- `fault.kill`: 0
- `null-jump`: 0
- `IPC storm`: 0
- `ring overflow`: 0
- `keyboard_gui FAIL`: 0
- `physical_keyboard_to_quil_text FAIL`: 0
- `keyboard.focus FAIL`: 0
- `EV_KEY FAIL`: 0

J) Files changed
- `docs/handoff/KEYBOARD_FOCUS_ROUTE_PROOF_V1.md` (new)

K) Next required autopilot
- `SHELL_INTERACTION_STATE_CONTRACT_V1`

## Notes / gaps for next phase
- Requested read target `docs/handoff/AGENT_HANDOFF_GP_CLOCK.md` is absent in this checkout.
- Required shortcut set in prompt includes `Alt+1-9`, `Shift+Alt+1-9`, `Ctrl+Alt+T`; these specific modifier-combo proofs are not currently expressed as dedicated gate markers in this lane.
- Current proof demonstrates shell-owned keyboard actions via `scancode_to_action` consume path and focus transitions, but not explicit Alt-combo contract markers.
