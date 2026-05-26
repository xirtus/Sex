# POINTER_NORMALIZER_CONTRACT_AUDIT_V1

A) PASS / FAIL / PARTIAL
- PASS (audit complete, contract frozen as-documented, no code patch required)
- Proof run result: `FINAL: PASS (276 gates proved, 116 skipped, 0 faults)`

B) Actual normalizer path
- Primary normalizer: `servers/sexinput/src/main.rs`
  - `fn normalize_pointer_report_v1(...)` at `servers/sexinput/src/main.rs:77`
- Primary caller:
  - `OP_USB_MOUSE_REPORT` handler in `servers/sexinput/src/main.rs:281` invokes normalizer at `servers/sexinput/src/main.rs:394`
- Upstream producers into this path:
  - `servers/sexusb/src/main.rs` sends `OP_USB_MOUSE_REPORT (0x260)` to sexinput slot 9 (`servers/sexusb/src/main.rs:250`, `servers/sexusb/src/main.rs:263`)
  - Synthetic proof path in sexinput also emits the same route markers
- Downstream consumer:
  - sexinput forwards normalized tuples as `OP_HID_EVENT (0x202)` to shell (`servers/sexinput/src/main.rs:156`, `servers/sexinput/src/main.rs:440`)

C) Input report contract (current code truth)
1. Ingress wire op
- `OP_USB_MOUSE_REPORT = 0x260` (`servers/sexinput/src/main.rs:18`, `servers/silk-shell/src/main.rs:3965`)

2. Accepted payload forms at sexinput ingress
- Compatibility pre-normalized HID tuple form:
  - `arg0 = class` where class is one of `EV_REL|EV_ABS|EV_BTN`
  - `arg1/arg2` already carry event payload
  - sexinput forwards directly as `OP_HID_EVENT` (`servers/sexinput/src/main.rs:307-356`)
- Packed pointer-report form (normalizer input):
  - `arg1 = buttons (u8)`
  - `arg2` packed axes:
    - relative mode (`is_abs=0`):
      - `dx = (arg2[7:0] as i8)`
      - `dy = (arg2[15:8] as i8)`
      - `wheel = (arg2[23:16] as i8)`
    - absolute mode (`is_abs=1` bit at `arg2[32]`):
      - `dx = (arg2[15:0] as i16)`
      - `dy = (arg2[31:16] as i16)`
      - `wheel = 0`
  - decode logic: `servers/sexinput/src/main.rs:358-363`

3. Raw report layout origins
- Boot mouse decode in sexusb:
  - len >=3 required; `buttons=byte0`, `dx=byte1 as i8`, `dy=byte2 as i8`, optional `wheel=byte3 as i8`
  - `servers/sexusb/src/main.rs:134-144`
- Tablet decode in sexusb:
  - len >=5 required; `buttons=byte0&0x07`, `abs_x=u16le(byte1..2)`, `abs_y=u16le(byte3..4)`
  - `servers/sexusb/src/main.rs:171-185`

4. Button bit mapping in normalizer
- Normalizer masks to low 3 bits: `buttons & 0x07` (`servers/sexinput/src/main.rs:83`)
- Edge loop maps bits to button IDs:
  - bit0 -> button 1 (left)
  - bit1 -> button 2 (right)
  - bit2 -> button 3 (middle)
  - `servers/sexinput/src/main.rs:137-147`

5. dx/dy sign and clamp behavior
- dx/dy sign decode:
  - relative path: signed i8 -> i16 -> i32 (`servers/sexinput/src/main.rs:361-362`, `87-88`)
  - absolute path: signed i16 -> i32 (`servers/sexinput/src/main.rs:361-362`, `87-88`)
- No clamping in normalizer; shell applies pointer filtering/clamp on consumption (`apply_rel_pointer`, `normalize_abs_coord`/`process_abs_tablet`)

6. Relative vs absolute semantics
- `report.is_abs=true` emits `EV_ABS` with absolute coordinates only when changed (`servers/sexinput/src/main.rs:94-114`)
- `report.is_abs=false` emits `EV_REL` only when `(dx!=0 || dy!=0)` (`servers/sexinput/src/main.rs:116-131`)

7. Wheel/scroll behavior
- Wheel is parsed into `HidPointerRawReport.wheel` but normalizer does not emit wheel event class in V1 (`servers/sexinput/src/main.rs:150-151`)
- Scroll policy remains deferred/non-normalized in this lane

8. Malformed/short report behavior
- At sexusb decode layer:
  - boot mouse len<3 => drop (`None`) (`servers/sexusb/src/main.rs:135-137`)
  - tablet len<5 => drop (`None`) (`servers/sexusb/src/main.rs:176-178`)
- At sexinput normalizer layer:
  - receives already-decoded packed fields; no explicit `report_len` check in normalizer
  - no dedicated malformed-report marker family observed in this lane

D) Output `OP_HID_EVENT` contract
1. Opcode/event class
- `OP_HID_EVENT = 0x202` (`servers/sexinput/src/main.rs:17`, `servers/silk-shell/src/main.rs:3964`)
- Event classes from sex-pdx:
  - `EV_KEY=1`, `EV_REL=2`, `EV_ABS=3`, `EV_BTN=4` (`crates/sex-pdx/src/lib.rs:132-135`)

2. Arg mapping emitted by normalizer
- `EV_REL`: `arg0=dx`, `arg1=dy`, `arg2=EV_REL` (`servers/sexinput/src/main.rs:127-129`)
- `EV_ABS`: `arg0=abs_x`, `arg1=abs_y`, `arg2=EV_ABS` (`servers/sexinput/src/main.rs:111-112`)
- `EV_BTN`: `arg0=button_id(1..3)`, `arg1=pressed(0|1)`, `arg2=EV_BTN` (`servers/sexinput/src/main.rs:141-143`)

3. Button down/up edge semantics
- XOR edge detection against `last_buttons`: only changed bits emit `EV_BTN` (`servers/sexinput/src/main.rs:84`, `136-147`)
- Repeated stable button state emits no repeated btn events (edges only)

4. Ordering guarantee
- Movement (`EV_ABS`/`EV_REL`) emitted before `EV_BTN` edges to keep click position current (`servers/sexinput/src/main.rs:92-94`, `134-135`)

E) Ownership boundaries (confirmed)
- `sexinput`/`sexusb`: production + decode + normalization only
- `silk-shell`: pointer state, focus/click/drag/resize policy ownership (`servers/silk-shell/src/main.rs:23802-24069`)
- `sexdisplay`: renderer/compositor ownership only (no normalizer policy ownership)
- No kernel/ABI/opcode contract change performed in this audit

F) Proof markers/gates found
1. Normalizer/route markers
- `[sexinput.usb_mouse.normalize.start]`
- `[sexinput.usb_mouse.normalize.ok]`
- `[sexinput.pointer.send] class=...`
- `[silk-shell.pointer.recv] class=...`

2. Motion/click/drag markers
- motion: `[silk-shell.rel.recv]`, `[shell.hid.rel.live]`, `[shell.cursor.move]`
- click: `[silk-shell.click.down]`, `[silk-shell.click.up]`, `[shell.click.real.target]`
- drag: `[shell.drag.threshold]`, `[shell.interact.drag.end]`

3. Relevant gates in this run
- `shell_interaction_contract PASS`
- `shell_interaction_pointer_no_focus_mutation PASS`
- `atlas_phase_e3_drag_begin_marker PASS`
- `atlas_phase_e4d_real_pointer_drop PASS`
- `faults_zero PASS`

G) Gaps before USB producer work
1. Malformed-report proof gap
- No explicit negative proof markers for malformed/short report drops at normalizer contract layer.

2. Scroll/wheel contract gap
- wheel parsed but no `EV_WHEEL`/scroll event class emitted in V1; policy intentionally deferred.

3. Mixed-source ambiguity risk
- Shell still has `OP_USB_MOUSE_REPORT` handling path (`servers/silk-shell/src/main.rs:22585`) alongside normalized `OP_HID_EVENT`; normalizer lane is clear, but dual ingress can blur ownership evidence if both active.

4. Absolute-range contract clarity gap
- Tablet absolute range normalization/clamp lives in shell path; normalizer emits raw signed i16 ABS values without a separate range contract marker.

5. Physical button-layout variability
- Current bit mapping is frozen to low 3 bits; additional HID button pages/usages are not covered.

H) Proof command + log path
- Command: `./scripts/run_daily_driver_proof.sh /tmp/pointer_normalizer_contract_audit_v1.log`
- Log: `/tmp/pointer_normalizer_contract_audit_v1.log`

I) Fault scan
Required token scan result in `/tmp/pointer_normalizer_contract_audit_v1.log`:
- `#PF`: 0
- `#GP`: 0
- `panic`: 0
- `fault.kill`: 0
- `null-jump`: 0
- `IPC storm`: 0
- `ring overflow`: 0
- `pointer FAIL`: 0
- `cursor FAIL`: 0
- `click FAIL`: 0
- `drag FAIL`: 0
- `shell_interaction FAIL`: 0
- `normalizer FAIL`: 0
- `hid FAIL`: 0

J) Files changed
- `docs/handoff/POINTER_NORMALIZER_CONTRACT_AUDIT_V1.md` (new)

K) Next required autopilot
- `CURSOR_VISIBLE_MOTION_PROOF_V1`

## STOP FIRST checks
- No STOP FIRST trigger required in this audit (no kernel edits, no sex-pdx edits, no ABI/opcode changes, no USB/XHCI implementation changes, no sexdisplay changes, no shell policy redesign).
