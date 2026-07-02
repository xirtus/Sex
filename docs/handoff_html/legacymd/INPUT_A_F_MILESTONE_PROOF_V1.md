# INPUT_A_F_MILESTONE_PROOF_V1

Date: 2026-05-13  
Scope: Commit 2 docs-only input milestone evidence (no code/build/runtime changes)

## A. USB Tablet Stability + Click/Drag Lifecycle

### GTK lane
```bash
./scripts/entrypoint_build.sh

LOG=/tmp/sexos_input_tablet_click_drag_v1.log
rm -f "$LOG"

qemu-system-x86_64 \
  -M q35 \
  -m 512M \
  -cpu max,+pku \
  -cdrom ./sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci \
  -device usb-tablet,bus=xhci.0 \
  -serial file:"$LOG" \
  -display gtk \
  -boot d
```

### Evidence summary
- synthetic marker present:
  - `[frame.light.zoom.synthetic.skip] reason=disabled`
- tablet stream survived ring wrap with requeue continuity
- click down/up reached shell and hit-test target marker recorded
- no runtime faults observed in this run

### Counts (tablet/click lane)
- `sexusb.tablet.active=61`
- `sexusb.xhci.intr_ring.wrap=4`
- `sexusb.tablet.requeue.doorbell=61`
- `sexinput.pointer.raw=27`
- `shell.cursor.final.send=17`
- `sexdisplay.cursor.draw=16`
- `shell.click.real.target=3`
- `shell.interact.drag=0`
- `shell.frame.rim.drag=0`
- `faults=0`

### Key marker snippets (tablet/click lane)
- `[silk-shell.pointer.recv] class=EV_BTN btn=1 pressed=true`
- `[shell.click.real.target] x=456 y=321 target=201 kind=app`
- `[silk-shell.pointer.recv] class=EV_BTN btn=1 pressed=false`

### Verdict
- Tablet stability: **PASS**
- Click lifecycle (down/up/target): **PASS**
- Drag: **NOT PROVEN** in this run

## B. USB Keyboard Proof

### GTK lane
```bash
LOG=/tmp/sexos_usb_keyboard_proof_v1.log
rm -f "$LOG"

qemu-system-x86_64 \
  -M q35 \
  -m 512M \
  -cpu max,+pku \
  -cdrom ./sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci \
  -device usb-kbd,bus=xhci.0 \
  -device usb-tablet,bus=xhci.0 \
  -serial file:"$LOG" \
  -display gtk \
  -boot d
```

### Observed keyboard route markers
- `[sexusb.xhci.config.hid_boot_keyboard.found]`
- `[sexusb.xhci.config.intr_ep.keyboard]`
- `[sexusb.hid.bind] role=keyboard`
- `[sexusb.kbd.found]`
- `[sexusb.hid.keyboard.continuous.start]`
- `[sexusb.kbd.raw]`
- `[sexinput.kbd.recv]`
- `[sexinput.key.ev_key.down code=0x1c]`
- `[sexinput.key.ev_key.up code=0x1c]`

### Counts (keyboard lane)
- `kbd=24`
- `keyboard=22`
- `key=112`
- `spindle=30`
- `faults=0`

### Caveat
Current grep evidence does not yet show a clear downstream shell/Spindle text-receipt marker (for example a definitive shell key-recv to app text append path).

### Verdict
- USB keyboard: **PARTIAL PASS**
- Proven: device enumeration/bind/report flow to `sexinput` and `EV_KEY` down/up emission
- Pending: downstream shell/Spindle text proof

## C. PS/2 Fallback Proof Status

### Verdict
- **NOT RUN / NOT NEEDED YET**

Rationale: USB keyboard hardware path to `sexinput` is already proven in this milestone. PS/2 fallback is deferred until/unless a separate gate requires it.

## D. Button/Click Lifecycle

### Verdict
- **PASS** for button down/up target path (as shown in section A)
- Drag remains **NOT PROVEN** in this run

## E. Synthetic GUI Proofs Default-Off Check

### Evidence
- Present (expected once):
  - `[frame.light.zoom.synthetic.skip] reason=disabled`
- Not observed in this milestone evidence:
  - `frame.light.zoom.synthetic.trigger`
  - `frame.light.zoom.synthetic.begin`
  - `frame.light.zoom.synthetic.click`
  - `frame.light.zoom.synthetic.done`

### Verdict
- Synthetic zoom default-off: **PASS**

## F. Final Verdict / Remaining Risks

### Milestone status
- A tablet stability: **PASS**
- D click lifecycle (down/up target): **PASS**
- Drag: **NOT PROVEN**
- E synthetic zoom default-off: **PASS**
- B USB keyboard: **PARTIAL PASS** (device -> sexinput `EV_KEY` proven; downstream shell/Spindle text proof pending)
- C PS/2 fallback: **NOT RUN / NOT NEEDED YET**

### Remaining risk
- Do not claim USB keyboard path as 100% complete until explicit downstream shell/app text markers are proven in a follow-up run.
