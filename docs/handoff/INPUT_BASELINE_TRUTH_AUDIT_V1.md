# INPUT_BASELINE_TRUTH_AUDIT_V1

A) PASS / FAIL / PARTIAL
- PASS (baseline audit complete; no code patch needed)
- Daily gate result: `FINAL: PASS (272 gates proved, 115 skipped, 0 faults)`

B) Exact current keyboard route
1. IRQ/scancode source
- `kernel/src/interrupts.rs`: IRQ1 handler (`idt[0x21]`) reads PS/2 scancode and enqueues into `INPUT_RING` with marker `[ps2.input_ring.enqueue]`.
2. sexinput event construction
- `servers/sexinput/src/main.rs`: polls `pdx_try_listen_raw(SLOT_INPUT)` for type `0x201`, derives `code=(scancode & 0x7F)`, `value=press/release`, emits `OP_HID_EVENT (0x202)` with `arg2=EV_KEY`.
3. PDX destination
- Current code path sends keyboard HID to `SLOT_SHELL` (silk-shell), not directly to sexdisplay.
4. sexdisplay handling
- No primary keyboard dispatch owner in current baseline route; sexdisplay remains renderer/compositor surface owner, while key routing policy is in silk-shell.
5. focused-window/shell consumption
- `servers/silk-shell/src/main.rs` `handle_hid_event(EV_KEY,...)`: reserved UI action consume first (`[shell.kbd.ui.*]`), then forward by focus to app slots (`SLOT_QUIL`, `SLOT_LINEN`, `SLOT_SPINDLE`) with `OP_HID_EVENT`.
6. proof markers/gates currently verifying keyboard path
- Runtime markers seen: `[ps2.irq1.entry]`, `[silk-shell.key.recv]`, `[silk-shell.key.route]`, `[shell.kbd.ui.focus]`.
- Gate rows: `quil_keyboard PASS`, `keyboard_gui SKIP not_requested` (explicit-sentinel policy).

C) Exact current cursor/pointer route
1. synthetic/raw source
- `sexinput` receives local USB pointer reports (`OP_USB_MOUSE_REPORT`) and also synthetic proof events.
2. normalizer function/path
- `normalize_pointer_report_v1(HidPointerRawReport, ...)` emits normalized `EV_ABS`/`EV_REL` + `EV_BTN` edge events.
3. OP_HID_EVENT emission
- `send_shell_hid_event(... OP_HID_EVENT ...)` to `SLOT_SHELL`; comments explicitly state OP_USB_MOUSE_REPORT is not forwarded as pointer control.
4. silk-shell pointer state update
- `handle_hid_event` handles `EV_ABS` (`process_abs_tablet`), `EV_REL` (`apply_rel_pointer`), `EV_BTN` (`POINTER_BUTTONS` update).
5. click focus handling
- On left down edge: `[silk-shell.click.down]` + `click_hit_test_and_focus(...)` + `[shell.click.real.target]`.
6. drag handling
- Drag candidate + threshold + interaction transitions; markers include `[shell.drag.candidate]`, `[shell.drag.threshold]`, `[shell.interact.drag.end]`.
7. proof markers/gates currently verifying pointer path
- Runtime markers seen: `[silk-shell.pointer.recv]`, `[silk-shell.click.down/up]`, `[shell.click.real.target]`, `[shell.interact.drag.begin/end]`.
- Gate rows: `atlas_phase_e3_drag_begin_marker PASS`, `atlas_phase_e4d_real_pointer_drop PASS`.

D) Proof command used
- `./scripts/run_daily_driver_proof.sh /tmp/input_baseline_truth_audit_v1.log`
- Runner internally calls `./scripts/daily_driver_master_gate.sh` on same log.

E) Proof log path
- `/tmp/input_baseline_truth_audit_v1.log`

F) Marker summary (baseline)
- Keyboard route evidence present:
  - `[ps2.irq1.entry]`
  - `[silk-shell.key.recv]`
  - `[silk-shell.key.route]`
  - `[shell.kbd.ui.focus]`
- Pointer/click/drag evidence present:
  - `[silk-shell.pointer.recv]`
  - `[silk-shell.click.down]` / `[silk-shell.click.up]`
  - `[shell.click.real.target]`
  - `[shell.interact.drag.begin]` / `[shell.interact.drag.end]`
- Gate summary:
  - PASS gates: 272
  - FAIL gates: 0
  - SKIP gates: 115

G) Fault scan result (required tokens)
- `#PF`: 0
- `#GP`: 0
- `panic`: 0
- `fault.kill`: 0
- `null-jump`: 0
- `IPC storm`: 0
- `ring overflow`: 0
- `monotonic.visible ok=0`: 0
- `keyboard_gui FAIL`: 0
- `cursor FAIL`: 0
- `pointer FAIL`: 0
- `click FAIL`: 0
- `drag FAIL`: 0

H) Current baseline gaps
1. Keyboard route docs expectation mismatch
- Current real route is `IRQ -> INPUT_RING -> sexinput -> silk-shell -> focused app`; not `... -> sexdisplay -> focused window`.
2. `keyboard_gui` gate is sentinel-gated
- In this run: `SKIP not_requested` (no explicit keyboard GUI begin marker).
3. Physical keyboard-to-Quil proof remains environmental SKIP
- `physical_keyboard_to_quil_text SKIP` despite QMP sendkey injection; done marker absent in this boot window.
4. Click/drag proof is present via shell/Atlas markers, but broad standalone gate rows named `click`/`drag` do not exist as independent top-level gate labels.

I) Next required autopilot
- `KEYBOARD_FOCUS_ROUTE_PROOF_V1`

J) Files changed
- `docs/handoff/INPUT_BASELINE_TRUTH_AUDIT_V1.md` (new)

K) STOP FIRST items encountered
- None triggered (no edits to `kernel/src/interrupts.rs`, `kernel/src/init.rs`, `crates/sex-pdx`, ABI/opcode layouts, framebuffer ownership, USB/XHCI, gesture policy, scheduler/time/PKRU).

## Notes
- Requested read target `docs/handoff/AGENT_HANDOFF_GP_CLOCK.md` was not present in this checkout during audit (`No such file or directory`).
