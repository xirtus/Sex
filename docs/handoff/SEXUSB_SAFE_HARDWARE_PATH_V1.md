# SEXUSB_SAFE_HARDWARE_PATH_V1

Date: 2026-07-05
Scope per phase: exactly ONE boundary. Never combine xHCI + HID parsing +
pointer policy + compositor in one patch. Kernel, sex-pdx, silk-shell,
sexdisplay, build spec: FORBIDDEN in every phase below (STOP FIRST to touch).
No Bluetooth. No gestures before Phase 6. No surface protocol changes.
No backing-buffer/shared-memory redesign.

---

## A. Current Reality Audit (from live `logs/qemu-latest.log`, 2026-07-05)

Much further along than the phase names imply. QEMU lane
(`nec-usb-xhci` + `usb-tablet`, gate_0_2.sh:363-364) already proves:

### Working today (marker evidence)
- **xHCI bring-up complete:** `[sexusb.xhci.map.ok]` → caplength/hciversion/
  hcsp1/hcc1 probe → reset → cmd ring 64 TRBs + event ring 64 TRBs + DCBAA +
  ERST alloc'd, CRCR write verified by readback (`crcr.write.ok wrote=0x1faf0001
  readback=0x1faf0001`) → `run.ok` rs=1 halted=0 → noop command completes cc=1.
- **Discovery:** 8-port scan `[sexusb.xhci.port] port=N connected= speed=`,
  `[sexusb.ports.collect] count=1 first=5` (usb-tablet at port 5, speed 3).
- **Minimal enum:** enable_slot → slot 1 → input/device context build with
  full ICC audit (`icc_audit.*` raw32 dumps, QEMU cross-check icc0_ok=true) →
  `address_device.complete.ok` cc=1 → `address_device.state.ok`.
- **HID interrupt-IN:** endpoint configured
  (`[usb.hid.endpoint.config] type=interrupt_in`), tablet reports flowing all
  boot long: `[sexusb.tablet.abs] x=13772 y=696 buttons=0` — values sane,
  monotonic under QMP moves (earlier "abs stream garbage" not reproducing in
  current tier; multi-TRB fix apparently effective).
- **Producer chain:** sexusb → `OP_USB_MOUSE_REPORT (0x260)` → sexinput
  `handle` scales raw 0..32767 → screen px, clamps deltas
  (`[usb.tablet.delta.clamp]`, `[usb.tablet.abs.delta]`), forwards to shell
  via `OP_HID_EVENT (0x202)`.

### Gaps
1. **No USB keyboard:** `[sexusb.hid.bind.summary] keyboard_ep=none
   pointer_ep=set pointer_role=tablet`. QEMU lane has no `usb-kbd` device;
   working keyboard is the PS/2 IRQ1-drain path. `OP_USB_KEYBOARD_REPORT
   (0x261)` opcode reserved but unused.
2. **Single-device only:** `SingleHidBind` struct (sexusb:189) — one slot,
   one pointer. No boot-protocol relative mouse lane (struct
   `BootMouseReport` at sexusb:119 exists but tablet is the only tested role).
3. **QEMU-only:** all evidence is nec-usb-xhci. No real-hardware run, no
   scratchpad buffers exercised (`diag.scratchpad max=0`), no MSI/interrupt
   path (polling).
4. **No touchpad:** no abs-contact model, no report-descriptor parsing at all
   (boot protocol assumptions throughout).
5. Current tier keyboard gate FAIL (`FIRST_MISSING_KEYBOARD:
   [sexinput.keyboard.send]`) — pre-existing input-lane state, tracked in
   SILK_WINDOW_MOVE_TEXT_INPUT_CURRENT_TIER_V1, not a USB gap per se.

### Ownership (already correct, must stay)
- **sexusb:** xHCI + USB transport + boot-protocol report extraction only.
  Emits `OP_USB_MOUSE_REPORT`/`OP_USB_KEYBOARD_REPORT` to sexinput.
- **sexinput:** report → pointer/keyboard policy (scaling, clamping,
  synthetic proofs); forwards `OP_HID_EVENT` to silk-shell.
- **silk-shell / sexdisplay:** untouched by this entire path.

---

## B. Phase Table

| # | Phase | Status today | New boundary proved | Files allowed |
|---|-------|--------------|--------------------|---------------|
| 1 | USB_HOST_DISCOVERY_V1 | ~DONE | deterministic port census marker + gate | sexusb, gate script |
| 2 | USB_XHCI_MINIMAL_ENUM_V1 | ~DONE | one enum summary line + no-device negative | sexusb, gate script |
| 3 | USB_HID_BOOT_MOUSE_REPORT_V1 | struct exists, untested | relative boot-mouse reports (usb-mouse lane) | sexusb, gate script |
| 4 | USB_HID_POINTER_PRODUCER_V1 | working (tablet) | producer contract hardening + malformed-report rejection | sexinput, gate script |
| 5 | TOUCHPAD_ABS_CONTACT_V1 | absent | single-contact abs model, proof-only | sexusb, sexinput (2 max) |
| 6 | TRACKPAD_GESTURES_V1 | DEFERRED | — none until 1-5 gated — | — |

Forbidden in ALL phases: `kernel/**`, `crates/sex-pdx/**`,
`servers/silk-shell/**`, `servers/sexdisplay/**`, `sexos_build_spec.toml`,
any new PDX opcode beyond existing 0x260/0x261/0x202 (STOP FIRST).

---

## C. Phase Prompts (Codex-ready)

### Phase 1 — USB_HOST_DISCOVERY_V1 (close-out)
```
TASK: USB host discovery close-out.
ALLOWED: servers/sexusb/src/main.rs, scripts/gate_0_2.sh (or new scripts/usb_path_gate.sh).
FORBIDDEN: kernel, sex-pdx, sexinput, silk-shell, sexdisplay, build spec.
Port scan already emits [sexusb.xhci.port] per port and [sexusb.ports.collect].
Add ONE deterministic summary after the scan:
  [sexusb.discovery.summary] ports=<n> connected=<n> first=<port|none> ok=1
derived from the same scan state (no constants).
NEGATIVE TEST: boot a lane variant with usb-tablet removed → summary must say
connected=0 first=none ok=1 and sexusb must reach its idle listen loop
(marker [sexusb.*.idle or existing listen marker]) — no spin, no timeout fault.
GATE: usb_discovery row — PASS on summary ok=1 present exactly once; FAIL on
absence or fault markers in sexusb lane.
STOP FIRST if: summary requires reading registers after Run/Stop transitions
not already performed, or any MMIO map change.
```

### Phase 2 — USB_XHCI_MINIMAL_ENUM_V1 (close-out)
```
TASK: xHCI minimal enum summary + negative path.
ALLOWED: servers/sexusb/src/main.rs, gate script.
FORBIDDEN: kernel, sex-pdx, sexinput, silk-shell, sexdisplay, build spec.
Chain already passes: enable_slot → address_device (cc=1) → hid endpoint
config. Add ONE summary emitted only after the chain completes:
  [sexusb.enum.summary] slot=<id> port=<p> speed=<s> ep_type=interrupt_in ok=1
On ANY step failure emit [sexusb.enum.summary] ok=0 stage=<name> cc=<code>
and continue to idle listen (never halt, never retry-spin — bounded retries
only if a bounded retry already exists).
NEGATIVE TEST: no-device lane → NO enum.summary ok=1, no address_device
markers, discovery summary connected=0, PD alive at end of lane.
GATE: usb_enum row — PASS: exactly one summary ok=1 in device lane AND zero
enum markers in no-device lane. FAIL: ok=0, duplicate summaries, or fault.
STOP FIRST if: fixing a failure requires touching command-ring allocation
sizes/layout (that is a ring redesign, own phase).
```

### Phase 3 — USB_HID_BOOT_MOUSE_REPORT_V1
```
TASK: boot-protocol RELATIVE mouse lane (usb-mouse, not usb-tablet).
ALLOWED: servers/sexusb/src/main.rs, gate script (new lane variant with
-device usb-mouse,bus=xhci.0 replacing usb-tablet).
FORBIDDEN: kernel, sex-pdx, sexinput (BootMouseReport wire format 0x260 must
not change), silk-shell, sexdisplay, build spec.
BootMouseReport struct exists (sexusb:119). Wire it: when bound HID role is
relative mouse (bind summary pointer_role=mouse), parse 3-4 byte boot report
(buttons, dx i8, dy i8, optional wheel) and emit via existing
OP_USB_MOUSE_REPORT with a role flag ALREADY representable in the existing
args — if the current 0x260 packing cannot distinguish abs/rel, STOP FIRST
(opcode contract change).
MARKERS: [sexusb.mouse.rel] dx=<d> dy=<d> buttons=<b> (budgeted: first 4 then
power-of-two summaries, per PERF_LOG_NOISE_ABLATION_V1 discipline);
[sexusb.hid.bind.summary] must show pointer_role=mouse.
NEGATIVE TEST: tablet lane unchanged — pointer_role=tablet, zero mouse.rel
markers; mouse lane emits zero tablet.abs markers.
GATE: usb_boot_mouse row — PASS: mouse lane has bind pointer_role=mouse +
≥1 mouse.rel after QMP button/move inject + no tablet.abs; tablet lane
regression-clean.
STOP FIRST if: 0x260 packing can't carry rel/abs distinction; or usb-mouse
enumerates at different speed requiring new endpoint math.
```

### Phase 4 — USB_HID_POINTER_PRODUCER_V1 (hardening)
```
TASK: sexinput producer contract hardening.
ALLOWED: servers/sexinput/src/main.rs, gate script.
FORBIDDEN: kernel, sex-pdx, sexusb, silk-shell, sexdisplay, build spec.
Existing path: OP_USB_MOUSE_REPORT → scale 0..32767 → clamp → OP_HID_EVENT.
Add validation BEFORE scaling: reject reports with raw coords outside
0..=32767 or unknown button bits, emit
  [sexinput.usb.report.reject] x=<raw> y=<raw> buttons=<b> reason=<range|buttons>
(budgeted) and drop — never forward garbage to shell. Keep the existing
delta clamp as second line of defense.
Emit once at startup: [sexinput.usb.contract] op=0x260 raw_max=32767 ok=1.
NEGATIVE TEST: synthetic-proof build injects one out-of-range report
(x=40000) through the same handle() path → reject marker fires, no
OP_HID_EVENT emitted for it (assert via absence of matching hid event marker),
next valid report flows normally.
GATE: usb_pointer_producer row — PASS: contract marker present, zero rejects
in clean lane, exactly 1 reject + continued flow in synthetic-negative lane.
STOP FIRST if: hardening requires changing OP_HID_EVENT packing (shell
contract) — that spans two domains.
```

### Phase 5 — TOUCHPAD_ABS_CONTACT_V1 (proof-only model)
```
TASK: single-contact absolute touch model, proof-only. NO gestures.
ALLOWED: servers/sexusb/src/main.rs, servers/sexinput/src/main.rs — the
two-domain maximum; touch NOTHING else.
FORBIDDEN: kernel, sex-pdx, silk-shell, sexdisplay, build spec, any report-
descriptor parser (boot/known-format only), any multi-contact state.
Model: ContactState { down: bool, x: u16, y: u16 } in sexinput, driven by the
EXISTING tablet abs stream (usb-tablet is the QEMU stand-in for an abs
touch surface — no new QEMU device). Transitions: press (buttons 0→1) =
contact.down, move-while-down = contact.drag, release = contact.up.
MARKERS (model-derived, never constants):
  [sexinput.contact.down] x= y=
  [sexinput.contact.drag] x= y= dx= dy= (budgeted)
  [sexinput.contact.up] x= y= total_dx= total_dy=
Behavior change to shell: NONE — OP_HID_EVENT output identical to today.
This phase proves the model only.
NEGATIVE TEST: release without press → no contact.up marker, state stays
idle, marker [sexinput.contact.spurious_up.ignored] fires once.
GATE: touch_contact row — PASS: QMP press-drag-release script produces
down→≥1 drag→up sequence in order with consistent coords; spurious-up lane
shows ignore marker and zero contact.up.
STOP FIRST if: tempted to emit new event types to shell (that is gesture
territory = Phase 6), or to parse HID report descriptors.
```

### Phase 6 — TRACKPAD_GESTURES_V1 (DEFERRED — do not prompt yet)
Blocked until Phases 1-5 rows are PASS in the master gate AND the contact
model has survived ≥1 week of lane runs. Gestures add shell policy surface
(scroll/swipe events) = new OP contract = automatic STOP FIRST. No prompt
is provided by design; writing one now would invite scope creep.

---

## D. Gate Wiring

New rows (gate_0_2.sh or scripts/usb_path_gate.sh):
`usb_discovery`, `usb_enum`, `usb_boot_mouse`, `usb_pointer_producer`,
`touch_contact`. Every row FAILs on any `KERNEL PAGE FAULT`,
`[sched.steal.reject]`, or sexusb fault marker in its lane (inherits
SCHEDULER_TICK_PD8_PF_FLAKE_V1 diagnostics). Marker budgets mandatory on all
per-report output — serial spam was the proven redraw bottleneck
(see input trace channel history).

## Implementation Status (2026-07-05, same day — approved)

Executed via `scripts/usb_path_gate.sh` (new, 3 lanes: tablet / no-device /
usb-kbd informational). Results:

| Row | Result | Evidence |
|-----|--------|----------|
| usb_discovery | PASS | `[sexusb.discovery.summary] ports=8 connected=1 first=5 ok=1` (new marker, both branches) |
| usb_discovery_negative | PASS | no-device lane: `connected=0 first=none ok=1`, no enum, no fault, PD parked in yield |
| usb_enum | PASS | pre-existing `[sexusb.enum.summary]` — Phase 2 was already complete, gate row added |
| usb_pointer_producer | PASS | new `[sexinput.usb.contract] op=0x260 raw_max=32767 buttons_mask=0x7`, validation added before scaling, zero rejects in clean lane |
| touch_contact | PASS | `[sexinput.contact.down/drag/up]` model in sexinput abs path, marker-only, OP_HID_EVENT unchanged |
| usb_kbd_lane (flag) | PASS | `keyboard_ep=set pointer_role=none`, `[sexusb.kbd.forward] key=0x4` → `[sexinput.usb_kbd.evkey] hid=0x4 sc=0x1e` — full USB keyboard path already worked, only the QEMU device was missing |
| lane1_fault_free | FAIL | pre-existing scheduler heap-corruption flake fired mid-lane — NOT a USB defect; see below |

**Phase 3 STOP FIRST resolved by audit:** 0x260 packing already carries
rel/abs in bit 32 of arg2 (`is_abs`, sexinput decode) and sexusb already
binds `role=mouse` for boot mice — Phase 3 needs only a usb-mouse lane
variant + gate row when wanted; no packing change, no STOP.

**Keyboard flag resolved:** both sides were already implemented
(sexusb decode+forward 0x261, sexinput hid→ps2 translation). The usb-kbd
lane in usb_path_gate.sh is informational (non-blocking) until promoted;
first PASS recorded above — promote to blocking on next gate revision.

**Scheduler flake interaction:** the tablet lane reproduced the
SCHEDULER_TICK_PD8_PF_FLAKE_V1 corruption twice. New `[sched.steal.reject]`
diagnostics caught it (payloads recorded in that handoff), and a
drain-and-retry fix was added to `Scheduler::tick` after the first run
exposed a stranded-runqueue deadlock. Remaining kernel halt is the
pre-existing upstream corruption, now under phase-2 investigation with
hard evidence — tracked in SCHEDULER_TICK_PD8_PF_FLAKE_V1.md, not here.

## Changelog
- 2026-07-05: Initial audit + phase prompts. Found phases 1-2 effectively
  complete and phase 4 working for the tablet role; reframed those as
  close-outs. Real gaps: USB keyboard bind (out of scope here — no phase
  requested), relative boot-mouse lane, contact model, real hardware.
- 2026-07-05 (later): Phases 1, 2, 4, 5 implemented + gated; keyboard flag
  lane wired and PASSING. Phase 3 unblocked (packing audit). Phase 6 still
  deferred by design.
- 2026-07-06: Phase 3 (`usb_boot_mouse`) and keyboard-lane promotion closed
  out in `scripts/usb_path_gate.sh` (gate-script-only change, no sexusb/
  sexinput code touched — audit's "no packing change needed" held).
  - New Lane 4 (`-device usb-mouse,bus=xhci.0`): QMP relative moves +
    one keypress-noop, asserts `pointer_role=mouse` in bind summary,
    `[sexinput.usb_mouse.decode.ok] ... is_abs=false` present, zero
    `[sexusb.tablet.abs]`. PASS confirmed (backup:
    `scripts/usb_path_gate.sh.bak.harness_usb_v1`).
  - `usb_kbd_lane` promoted from informational to blocking per the prior
    entry's own instruction ("promote to blocking on next gate revision").
  - 3 consecutive gate runs (`/tmp/sexos_usb_path_gate_v2..v4`): every row's
    marker logic verified correct (PASS when clean, correctly FAILs only
    when the pre-existing pd=8 `Scheduler::tick` PF flake fired — hit
    tablet lane once, kbd lane once, in different runs). Flake is
    kernel-side, forbidden to touch in this phase, tracked separately in
    `SCHEDULER_TICK_PD8_PF_FLAKE_V1.md` — not a USB regression.
  - One false-positive caught and reverted during this pass: an added
    tablet-lane negative check for `is_abs=false` mouse-decode markers
    misfired on the tablet's own transient zero-report at connect; removed,
    Phase 3 proof relies solely on Lane 4 instead.
  - Remaining real gap, unchanged: real hardware (QEMU-only), no MSI/
    interrupt path, no report-descriptor parsing. Phase 6 still deferred.
