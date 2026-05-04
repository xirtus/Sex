# USB Input — Current Status & Where to Continue

> Single-source-of-truth for the USB input situation.
> Last updated: 2026-05-03
> Offloaded from CLAUDE.md for quick continuation.

---

## The Situation in One Paragraph

The entire USB input pipeline is **code-complete and guest-proven**. The problem is
**upstream of the guest**: QEMU 11.0.0 does not deliver real host mouse movement or button
clicks to emulated USB HID devices (neither usb-mouse nor usb-tablet). The pipeline from
sexusb → sexinput → silk-shell → sexdisplay is verified working via synthetic proofs.
Physical USB button proof is blocked by the QEMU/SDL2 host layer.

---

## What's Proven (Working)

### Synthetic Proofs (env var `SEXOS_PROOFS_DISABLED` unset = enabled)

| Proof | Markers to grep |
|-------|-----------------|
| SilkBar clickable controls | `grep "shell.silkbar.click" /tmp/silkbar-click.log` |
| Drag-window proof | `grep -E "shell.drag.start\|shell.drag.move\|shell.drag.end" /tmp/drag-proof.log` |
| Click-focus chain | `grep "shell.click_focus" /tmp/click-focus-proof.log` |
| Top-strip render hash | `grep "silk.render_proof" /tmp/silk-render-proof.log` |

### Real USB (Non-Synthetic) Proven

- **QEMU usb-tablet HID detection** (commit 04566ab): interface config walk, report
  descriptor shape scan, SHORT_PACKET acceptance — all proven via boot markers.
- **Nonzero tablet position reports captured** in SDL X11 interactive session:
  ```
  [sexusb.hid.tablet.report] i=1 buttons=0x0 x=32741 y=9625 dx=127 dy=127
  [shell.pointer.usb_state.nonzero.ok] x=767 y=487 buttons=0x0 wheel=0 dx=127 dy=127
  ```
- **Mouse delta markers** `[sexinput.mouse.real.delta]` fire for real USB movement
  (not triggered by synthetic HID_EVENT path).
- **Keyboard cursor mode** (KEYBOARD_DEVICE_MODE_V1): arrow/WASD → EV_REL (8px step)
  via usb-kbd, proven for cursor movement.

---

## The Blocker (What's NOT Working)

### Physical USB Button Events

**Cannot prove real USB button clicks through the full pipeline.**
No `buttons=0x01` has ever been observed from a real USB device.

**Root cause chain:**
```
Host mouse movement/click
  → Linux kernel event subsystem
    → QEMU 11.0.0 (GTK or SDL backend)
      → Emulated usb-tablet or usb-mouse HID device
        → sexusb xHCI interrupt-IN polling  ← THIS IS WHERE IT DIES
```

**Why it dies — three independent confirmations:**

1. **QMP/HMP injection blocked:** `input-send-event` returns `{"return": {}}` but
   events go to PS/2 display layer, NOT USB HID. Confirmed: usb-mouse/tablet
   sees nothing.

2. **SDL2 XTest filter:** SDL2 filters synthetic XTest events (`send_event=True`).
   `xdotool` uses XTest → zero USB reports. VNC RFB `PointerEvent` also does NOT
   reach USB HID.

3. **HOST_INPUT_BACKEND_AUDIT_V1 (2026-05-04):** On a real local desktop with
   physical trackpad movement, both usb-mouse and usb-tablet produce ONLY idle
   reports (all zeros). GTK and SDL backends both fail. The problem is upstream
   of the guest — cannot fix in SexOS server code.

---

## What's Been Tried (Audits)

### TABLET_LIVENESS_TRACE_V1 (2026-05-04)
8 bounded markers across 4 servers tracing cursor pipeline.
**Non-interactive finding:** 15 reports forwarded, all dx=dy=0. QEMU 11.0.0
usb-tablet always reports (0,0) in headless env.
See `docs/handoff/TABLET_LIVENESS_TRACE_V1.md`.

### QEMU_INPUT_CONFIG_AUDIT_V1 (2026-05-04)
dev.sh audit complete. QEMU 11.0.0 not delivering non-idle coordinates to
usb-tablet. Guest pipeline proven healthy. Dead layer is outside guest.
**dev.sh flags discovered:**
- `QEMU_PRINT_CMD=1` — print exact argv, no launch
- `SEXUSB_QEMU_DEVICE=tablet-display-sdl` — adds `display=sdl` to usb-tablet
- `SEXOS_QEMU_NODEFAULTS=1` — adds `-nodefaults` (disables PS/2 input)
- `SEXOS_QEMU_DISPLAY=none` — for headless runs
See `docs/handoff/QEMU_INPUT_CONFIG_AUDIT_V1.md`.

### HOST_INPUT_BACKEND_AUDIT_V1 (2026-05-04)
Real desktop, physical trackpad. Both usb-mouse and usb-tablet produce only
idle reports on GTK and SDL backends. Problem confirmed upstream of guest.
See `docs/handoff/HOST_INPUT_BACKEND_AUDIT_V1.md`.

---

## The Workaround That Works

**SDL X11 window + physical mouse** is the only proven path to deliver real
USB events:

```bash
SDL_VIDEO_DRIVER=x11 SEXUSB_QEMU_DEVICE=tablet ./dev.sh run
```

- Produces a visible X11 window (confirmed via `xdotool`)
- Host mouse events forwarded through SDL **do** reach the usb-tablet device
- Proof sequence: launch → wait for desktop → `xdotool search --name "QEMU"`
  → `xdotool mousemove --window $WID X Y` → `xdotool click 1`
- **Caveat:** First SDL window click is consumed by SDL grab (not forwarded).
  Second click = first real USB button event.

**Do NOT use** `-display gtk,grab-on-hover=on` — GTK steals keyboard focus,
stray keypresses open Limine config editor and prevent boot.

---

## The Fallback: Keyboard Cursor Mode (KEYBOARD_DEVICE_MODE_V1)

Since mouse input is blocked, we implemented keyboard cursor as a fallback:

```
sexusb (usb-kbd) → sexinput (HID usage→EV_REL) → silk-shell (cursor move)
```

- `SEXUSB_QEMU_DEVICE=kbd` → QEMU `-device usb-kbd,bus=xhci.0`
- `SEXOS_KEYBOARD_CURSOR=1` at build time enables arrow/WASD → EV_REL (8px step)
- sexusb: keyboard HID detection + forward via `OP_USB_KEYBOARD_REPORT=0x261`
- sexinput: decode + map HID usage IDs to EV_REL cursor movement
- No xHCI refactor (single-device mode). No kernel/ABI/renderer/display/shell changes.
- See `docs/handoff/KEYBOARD_DEVICE_MODE_V1.md`

---

## Where to Continue — Next Action

**Pick ONE:**

### Option A: Physical Mouse Proof (interactive)
Grab a real mouse, run in SDL X11 mode, click twice in the QEMU window.
Check for:
```
grep "shell.click_focus.down" /tmp/click-focus-proof.log
grep "shell.click.real" /tmp/click-focus-proof.log
```

### Option B: uinput Virtual Mouse (bypass XTest)
Create a Linux virtual input device via `/dev/uinput`. Events from uinput
appear as real device events to SDL, bypassing the XTest synthetic-event
filter. Full-chain proof without physical hardware.

### Option C: Re-enable Synthetic Click-Focus Proof
Set `USB_PROOF_DISABLE_SYNTH_CLICK = false` in source. Proves the click-focus
chain alongside the already-proven drag proof. Does NOT prove real USB button
path, but confirms the shell→display click delivery.

---

## Pipeline Architecture (Quick Reference)

```
QEMU usb-mouse (boot HID, relative, 4-byte)  OR  usb-tablet (absolute, 6-byte)
  → sexusb (PD7 @ 0x46000000): xHCI interrupt-IN polling, circular ring,
                                SHORT_PACKET, tablet absolute→relative delta
  → sexinput (PD4 @ 0x43000000): normalize, clamp, send OP_USB_MOUSE_REPORT
  → silk-shell (PD3 @ 0x42000000): POINTER_X/Y/buttons, cursor move (0xEB),
                                    click-focus hit-test (0xED)
  → sexdisplay (PD1 @ 0x40000000): render surfaces, cursor z-top pass
```

### Key Opcodes
| Value | Name | Origin → Target |
|-------|------|-----------------|
| 0x260 | OP_USB_MOUSE_REPORT | sexinput → silk-shell |
| 0x261 | OP_USB_KEYBOARD_REPORT | sexusb → sexinput |
| 0xEB  | move surface | silk-shell → sexdisplay |
| 0xED  | focus surface | silk-shell → sexdisplay |
| 0xEC  | create surface | silk-shell → sexdisplay |

### Tablet Decode
- `decode_tablet_report(buf, len)`: parses 5 bytes (buttons, abs_x u16 LE, abs_y u16 LE)
- Delta: `dx = clamp(abs_x - prev_x, -128, 127)`, same for dy
- First report: zero delta (prevents initial position jump)
- Same PDX message format as boot mouse (OP_USB_MOUSE_REPORT = 0x260, packed_axes)
- **Key invariant:** tablet absolute positions (0..32767) are converted to relative
  deltas before reaching sexinput.

### xHCI Interrupt-IN Ring (sexusb)
- `INTR_TR_RING_SIZE = 16`. Slots 0–14 = Normal TRBs. Slot 15 = Link TRB.
- Link TRB with TC=1 toggles Consumer Cycle State on wrap.
- Never write all Normal TRBs to slot 0 — controller dequeue pointer advances.
- State: `intr_prod: u64`, `intr_pcs: u32 = 1`. Wrap at 15 → toggle `intr_pcs`.

### Click-Focus Guard
`CLICK_ACTIVE` bool prevents repeat focus on held button. Rising edge only
(button down, not held). `try_set_focus()` guards all focus write sites.
