# CURSOR_KEYBOARD_HANDOFF_CLOSEOUT_V1

## A) RESULT: CURRENT-TIER 100% (SYNTHETIC/CURRENT-TIER ROUTE), USB DEFERRED

FINAL integrated proof: `PASS (309 gates proved, 133 skipped, 0 faults)`  
Zero faults across all proof lanes.  
USB real HID report arrival honestly deferred.

---

## B) PHASE TABLE: AP1–AP15

| Phase | Handoff | Status | Summary |
|---|---|---|---|
| AP1 | INPUT_BASELINE_TRUTH_AUDIT_V1 | **PASS** | Keyboard/pointer route audit. 272 gates, 0 faults. |
| AP2 | KEYBOARD_FOCUS_ROUTE_PROOF_V1 | **PASS** | Keyboard focus route proven (PS/2→sexinput→shell→app). 272 gates, 0 faults. |
| AP3 | SHELL_INTERACTION_STATE_CONTRACT_V1 | **PASS** | Contract defined, mapped to current code. Docs-only. |
| AP4 | SHELL_INTERACTION_STATE_IMPL_V1 | **PASS** | Shell interaction state machine implemented + markers. |
| AP5 | POINTER_NORMALIZER_CONTRACT_AUDIT_V1 | **PASS** (audit) / partial closeout deferred | Contract frozen as-documented. 276 gates, 0 faults. Closeout deferred/optional. |
| AP6 | CURSOR_VISIBLE_MOTION_PROOF_V1 | **PARTIAL-PASS** (lane) / **PASS** (AP15 integrated) | Cursor lane proofed (logical motion, bounds, no-focus-mutation). Covered by AP15. |
| AP7 | CLICK_FOCUS_DRAG_PROOF_PLAN_V1 | **PASS** | Proof plan complete. Docs-only. |
| AP8 | CLICK_FOCUS_DRAG_IMPL_V1 | **PASS** | Click/drag markers + gates added. Clean rerun: 285 gates, 0 faults. |
| AP9 | SURFACE_ID_LIFETIME_INPUT_SAFETY_V1 | **PASS** | 7 surface lifetime gates PASS. 292 gates, 0 faults. |
| AP10 | INPUT_ROUTE_NEGATIVE_TESTS_V1 | **PASS** | 8 negative test gates PASS. 298 gates, 0 faults. |
| AP11 | USB_HOST_DISCOVERY_V1 | **DISCOVERY COMPLETE** | Full XHCI enum/config/detect audit. Docs-only. |
| AP12 | USB_HID_KEYBOARD_REPORT_V1 | **PARTIAL** | Structurally wired (detect+shape scan PASS). No real keyboard reports (idle QEMU usb-kbd). |
| AP13 | USB_HID_BOOT_MOUSE_REPORT_V1 | **PARTIAL** | Structurally wired (detect+shape scan PASS). No real mouse reports (idle QEMU usb-mouse). |
| AP14 | USB_HID_POINTER_PRODUCER_V1 | **PARTIAL** | Structurally wired (detect+shape scan PASS, normalizer ready, shell recv ready). No real USB pointer reports. |
| — | USB_POINTER_REPORT_EVENT_UNBLOCK_V1 | **PREPARED / PARTIAL** | Operator probe script created. Three modes (evdev, gtk, usb-host). Event-ring alignment root-caused. Requires graphical QEMU or physical device. |
| — | USB_XHCI_EVENT_RING_TRANSFER_CONSUME_V1 | **PASS** (fix) / **PARTIAL** (reports) | 15 poll locations fixed to consume non-matching events. Reports still blocked by QEMU input barrier. |
| **AP15** | **INTEGRATED_CURSOR_KEYBOARD_SCENARIO_PROOF_V1** | **PASS** | **8 integrated gates PASS. 309 gates, 0 faults.** Pure host-side aggregation. |

---

## C) CURRENT-TIER 100% STATEMENT

### CLAIMED (100% current-tier for synthetic/current-tier route):

1. **Keyboard route proven** — PS/2 IRQ1 → INPUT_RING → sexinput → silk-shell → focused app.  
   Evidence: 59 `silk-shell.key.recv` markers. Gate: `integrated_keyboard_route PASS`.

2. **Cursor visible/logical motion proven** — EV_REL/EV_ABS ingress → pointer update → cursor surface.  
   Bounds clamped to display dimensions. No-focus-mutation proven (movement alone does not change focus).  
   Evidence: 8 `silk-shell.pointer.recv`, 2 `cursor.motion.bounds ok=1`.  
   Gates: `integrated_cursor_motion PASS`.

3. **Click focus proven** — Button down → hit-test → focus commit → button up with ok=1.  
   Evidence: `click.focus.proof.begin`, `click.focus.button.down/up`, hit-test, commit markers.  
   Gates: `integrated_click_focus PASS`.

4. **Drag lifecycle proven** — Drag candidate → threshold → begin → move → release → capture cleared.  
   Evidence: `drag.proof.begin`, `drag.capture.begin/move/release`, `drag.proof.done`.  
   Gates: `integrated_drag_lifecycle PASS`.

5. **Surface lifetime/dead-target guards proven** — Focus commit validated live target.  
   Dead focus/drag/hover cleared safely. Tombstone rejection active. Generation/ref tracking active.  
   Evidence: `surface.input_lifetime.begin`, focus_live, key_route_guard, click_target_guard, drag_target_guard, dead_clear, done markers.  
   Gates: `integrated_surface_lifetime PASS`.

6. **Negative input routes proven** — Unknown class silently ignored. Bad button silently ignored.  
   Button-up without capture silently ignored. Dead-target markers active.  
   Evidence: `input.negative.once`, unknown_class, bad_button, button_up_no_capture markers.  
   Gates: `integrated_negative_inputs PASS`.

7. **Zero faults** — 0 #PF, 0 #GP, 0 panic, 0 fault.kill, 0 null-jump, 0 IPC storm, 0 ring overflow.  
   Gates: `integrated_faults_zero PASS`.

8. **Integrated proof PASS** — All 8 integrated gates PASS in a single unified proof run.  
   309 gates proved, 133 skipped, 0 faults.

### NOT CLAIMED (explicitly deferred):

- **Real USB HID report arrival** — USB pointer producer lane runs but no hardware report arrives in headless QEMU.  
  `integrated_usb_real_report_deferred PASS` on honest deferral.
- **Physical USB mouse/keyboard complete** — Structurally wired (detect + shape scan PASS), but no real reports.
- **Multi-device USB** — Single xhci controller with single device probed.
- **Trackpad gestures** — Not implemented.
- **Scroll wheel** — Not implemented.
- **Malformed normalizer injection** — No injectable path at normalizer layer (sexusb drops short reports upstream).

---

## D) FINAL PROOF

**Command:**
```
./scripts/run_daily_driver_proof.sh /tmp/integrated_cursor_keyboard_scenario_proof_v1.log
```

**Log:** `/tmp/integrated_cursor_keyboard_scenario_proof_v1.log` (102,727 lines)

**Result:**
```
PASS gates: 309
FAIL gates: 0
SKIP gates: 133

FINAL: PASS (309 gates proved, 133 skipped, 0 faults)
```

**Integrated gate results:**

| Gate | Status |
|---|---|
| integrated_keyboard_route | PASS |
| integrated_cursor_motion | PASS |
| integrated_click_focus | PASS |
| integrated_drag_lifecycle | PASS |
| integrated_surface_lifetime | PASS |
| integrated_negative_inputs | PASS |
| integrated_faults_zero | PASS |
| integrated_usb_real_report_deferred | PASS |

**Fault scan:** CLEAN — zero #PF, #GP, panic, fault.kill, null-jump, IPC storm, ring overflow.  
6 stale markers (all mesh/clock-related, zero input-routing).

---

## E) USB STATUS

### What works:
- **XHCI init complete** — BAR0 map, stop/reset, ring alloc, DCBAAP/CRCR/ERST programming, Run/Stop.
- **Port enumeration** — Enable Slot, Address Device, GET_DESCRIPTOR chain (device/config/HID report), SET_CONFIG, SET_IDLE.
- **HID descriptor walk** — Interface class/subclass/protocol detection for keyboard, mouse, tablet.
- **HID report descriptor shape scan** — Keyboard shape (Usage Page 0x01, Usage Keyboard 0x06), mouse shape (Usage Page 0x01, Usage Mouse 0x02, X/Y 0x30/0x31), tablet shape.
- **Interrupt endpoint configured** — Configure Endpoint, Normal TRB armed (IOC=1), doorbell.
- **Event ring non-transfer consume fix** — All 15 poll locations correctly consume non-matching events.
- **Report decode paths** — `decode_boot_mouse_report` (3+ bytes), `decode_tablet_report` (5 bytes), boot keyboard decode.
- **PDX route wired** — `OP_USB_MOUSE_REPORT (0x260)` → sexinput normalizer → `OP_HID_EVENT (0x202)` → silk-shell.
- **Pointer normalizer ready** — `normalize_pointer_report_v1()` accepts packed pointer reports and emits normalized EV_ABS/EV_REL/EV_BTN.

### What's blocked:
- **No real USB HID reports** — Interrupt IN polls timeout. QEMU `-display none` with `-device nec-usb-xhci -device usb-tablet -device usb-kbd -device usb-mouse` does not generate USB HID interrupt reports in headless mode. The controller generates Transfer Events (confirmed via QEMU trace), but these are buffer-clear/non-data events or the data path from QEMU's HID emulation to the XHCI controller data buffer is incomplete in headless mode.
- **QMP/HMP sendkey/mouse_move routes** — These inject PS/2 IRQ1 events, not USB HID reports.
- **GTK/evdev operator probe** — Probe script prepared but not yet executed in graphical environment.

### Next for USB: `USB_POINTER_REAL_INPUT_OPERATOR_RETRY_V1`
- Requires graphical QEMU (`-display gtk`) or `-object input-linux,id=mouse,evdev=/dev/input/...` passthrough.
- Operator probe script: `scripts/usb_pointer_real_report_operator_probe.sh`.

---

## F) FILES CHANGED (AP15 CLOSEOUT)

- `docs/handoff/CURSOR_KEYBOARD_HANDOFF_CLOSEOUT_V1.md` — **new** (this file)

### Cumulative files changed across AP1–AP15 (production code only):

| File | Phases |
|---|---|
| `servers/silk-shell/src/main.rs` | AP4, AP6, AP8, AP9, AP10 |
| `servers/sexinput/src/main.rs` | AP5, AP12, AP14 |
| `servers/sexusb/src/main.rs` | AP12, AP13, AP14, XHCI fix |
| `scripts/daily_driver_master_gate.sh` | AP4, AP6, AP8, AP9, AP10, AP14, **AP15** |
| `scripts/run_daily_driver_proof.sh` | AP4, AP8, AP9, AP10, AP14 |
| `scripts/usb_pointer_real_report_operator_probe.sh` | USB unblock (new) |

No kernel, ABI, sex-pdx, or sexdisplay edits in any phase.

---

## G) RECOMMENDED NEXT

### Option A: Liveboot current-tier now (SAFE)
Safe to liveboot for current-tier UI/input if goal is:
- QEMU/synthetic current-tier cursor/keyboard verification
- Visual clock/window/click/drag checks
- Daily-driver gate run

Not a final proof of physical USB HID yet.

### Option B: AP5B/AP6B closeout polish (OPTIONAL)
- AP5 normalizer contract closeout (contract is frozen, marker is deferred/optional)
- AP6 cursor visible motion standalone rerun (already covered by AP15 integrated PASS)

### Option C: USB_POINTER_REAL_INPUT_OPERATOR_RETRY_V1 (NEXT BLOCKER)
Required for real USB HID report arrival proof.  
Needs graphical QEMU or evdev passthrough.  
Operator probe script is ready: `scripts/usb_pointer_real_report_operator_probe.sh`.

### Option D: TRACKPAD/SCROLL (LATER)
Deferred until real USB pointer reports arrive.

### Option E: Commit
```bash
git add scripts/daily_driver_master_gate.sh docs/handoff/INTEGRATED_CURSOR_KEYBOARD_SCENARIO_PROOF_V1.md docs/handoff/CURSOR_KEYBOARD_HANDOFF_CLOSEOUT_V1.md
git commit -m "input: add integrated cursor keyboard proof and closeout"
```
