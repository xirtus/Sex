# USB Input Pipeline Reference

> Referenced from CLAUDE.md (offloaded reference).

---

## Pipeline Architecture

```
QEMU usb-mouse (boot HID, relative, 4-byte reports)
  OR QEMU usb-tablet (absolute, 6-byte reports via interrupt-IN, decoded to relative deltas)
  → sexusb (PD7 @ 0x46000000): xHCI interrupt-IN polling, circular ring,
                                SHORT_PACKET acceptance, tablet absolute→relative delta
  → sexinput (PD4 @ 0x43000000): normalize, clamp, send OP_USB_MOUSE_REPORT to silk-shell
  → silk-shell (PD3 @ 0x42000000): update POINTER_X/Y/buttons, move cursor surface (0xEB),
                                    click-focus hit-test (0xED)
  → sexdisplay (PD1 @ 0x40000000): render surfaces, cursor z-top pass, arrow bitmap
```

### Tablet Decode Path (sexusb)

- `decode_tablet_report(buf, len) -> Option<TabletReport>`: parses 5 bytes (buttons, abs_x u16 LE, abs_y u16 LE)
- Static mut state: `PREV_ABS_X`, `PREV_ABS_Y`, `FIRST_TABLET_REPORT`
- Delta computation: `dx = clamp( abs_x - prev_x, -128, 127 )` (same for dy)
- First report: sets prev to current, sends zero delta (prevents initial position jump)
- Same PDX message format as boot mouse (OP_USB_MOUSE_REPORT = 0x260, packed_axes)
- **Key invariant:** tablet absolute positions (0..32767) are converted to relative deltas before reaching sexinput. sexinput and silk-shell see no difference from boot mouse reports.

---

## xHCI Interrupt-IN Transfer Ring (sexusb)

**Critical invariant (FIXED 2026-05-02):** Never write all Normal TRBs to ring slot 0.

After the xHCI processes slot 0 and the software re-writes slot 0 again, the controller
dequeue pointer is at slot 1. Ringing the doorbell makes the controller re-read slot 1
(not slot 0). If slot 1 has cycle=0, controller stops — all polls after the first stall.

**Fix in `servers/sexusb/src/main.rs`:**
- Ring layout: `INTR_TR_RING_SIZE = 16`. Slots 0–14 = Normal TRBs. Slot 15 = Link TRB.
- Link TRB: `d0/d1 = intr_ring_phys`, `d3 = (TRB_TYPE_LINK<<10) | TC | intr_pcs`.
  TC=1 causes xHCI to toggle its Consumer Cycle State on wrap.
- Poll loop state: `intr_prod: u64 = 0`, `intr_pcs: u32 = 1`.
- Each iteration: write Normal TRB at `intr_prod` with `intr_pcs`, ring doorbell, wait event.
- After event consumed: `intr_prod += 1`. If `intr_prod >= 15`: toggle `intr_pcs`,
  update Link TRB cycle bit to new `intr_pcs`, `intr_prod = 0`.
- Endpoint dequeue: `ep_deq = intr_ring_phys | 1` (DCS=1 matches initial `intr_pcs=1`).

---

## QEMU SDL/Tablet Notes

- SDL requires a left-click inside the window to grab host mouse. First click consumed by SDL (not forwarded to USB). Second click = first real USB button event.
- `dx`/`dy` events only arrive after grab in boot-mouse mode. For usb-tablet, absolute position reports arrive even without grab (QEMU SDL forwards absolute motion directly).
- **Key finding:** `SDL_VIDEO_DRIVER=x11` is required when DISPLAY is available but Wayland is default. Without this, SDL uses Wayland backend and creates no visible X11 window.
- Do NOT use `-display gtk,grab-on-hover=on` — GTK steals keyboard focus, stray keypresses open Limine config editor and prevent boot.
- Proof sequence: `SDL_VIDEO_DRIVER=x11 SEXUSB_QEMU_DEVICE=tablet ./dev.sh run`, wait for desktop, find window via `xdotool search --name "QEMU"`, inject mouse via `xdotool mousemove --window $WID X Y` and `xdotool click 1`.

### Known Blockers

- **Button injection blocked (confirmed 2026-05-03):**
  - SDL2 filters XTest synthetic events (`send_event=True`); `xdotool` uses XTest → zero USB reports
  - VNC RFB `PointerEvent` also does NOT reach USB HID in QEMU 11.0
  - QMP/HMP: already confirmed blocked
  - USB tablet decode code is correct; silk-shell click-focus code is correct
  - Only real physical mouse over SDL window can deliver button events

- **QEMU 11.0:** QMP/HMP input injection does NOT route to USB HID devices. Events consumed by PS/2 display layer only. Confirmed: `input-send-event` returns `{"return": {}}` but usb-mouse/tablet sees nothing.

- **Workaround discovered:** `SDL_VIDEO_DRIVER=x11` + `-display sdl` produces a visible X11 window (confirmed via `xdotool`). Mouse events from the host X11 desktop forwarded through SDL do reach the usb-tablet device. This enables proof in headless environments with Xvfb or similar.

---

## Keyboard Cursor Mode (KEYBOARD_DEVICE_MODE_V1)

USB keyboard replaces mouse as HID device (mouse only produces idle reports).
- `SEXUSB_QEMU_DEVICE=kbd` → QEMU `-device usb-kbd,bus=xhci.0`
- `SEXOS_KEYBOARD_CURSOR=1` at build time enables arrow/WASD → EV_REL (8px step)
- sexusb: keyboard HID detection + forward via `OP_USB_KEYBOARD_REPORT=0x261`
- sexinput: decode + map HID usage IDs to EV_REL cursor movement
- No xHCI refactor (single-device mode). No kernel/ABI/renderer/display/shell changes.
