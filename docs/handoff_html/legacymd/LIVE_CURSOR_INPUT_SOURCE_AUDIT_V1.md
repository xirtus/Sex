# LIVE_CURSOR_INPUT_SOURCE_AUDIT_V1

## A) RESULT

Root cause identified. No code change needed. One env-var change unblocks live cursor.

---

## B) TRUE SOURCE OF CENTER CURSOR (x=640 y=360)

Two places in silk-shell initialize `POINTER_X/Y` to `P.width/2, P.height/2`:

1. **`silk-shell:21984`** — cursor surface creation:
   ```rust
   let cursor_arg1 = ((P.height / 2) as u64) << 32 | (P.width / 2) as u64;
   ```
   Places the cursor surface at (640, 360) on boot. P.width=1280, P.height=720.

2. **`silk-shell:8878`** — first USB report arrival (`POINTER_USB_STATE_INIT` gate):
   ```rust
   POINTER_X = P.width / 2;  // = 640
   POINTER_Y = P.height / 2; // = 360
   ```
   First real report would set center, then deltas would move it.

Since no USB report ever arrives, `POINTER_USB_STATE_INIT` stays false.
Cursor surface stays at boot position (640, 360) forever.

---

## C) WHY LIVE HOST MOUSE DOES NOT MOVE THE CURSOR

### QEMU input barrier — the complete chain

Running:
```
SEXOS_QEMU_DISPLAY=sdl-grab SEXUSB_QEMU_DEVICE=tablet ./dev.sh
```

Generates:
```
-display sdl,grab-mod=lctrl-lalt
-device usb-tablet,bus=xhci.0         ← NO display= binding
```

**QEMU routing behavior:**
- SDL window mouse events → PS/2 i8042 controller (default QEMU route)
- USB tablet (`usb-tablet,bus=xhci.0`) → not connected to any input source
- USB tablet generates **zero** Transfer Events (interrupt-IN ring stays idle)
- `sexusb.xhci.event.transfer.intr_in` never fires

**SexOS PS/2 gap:**
- SexOS handles PS/2 keyboard (IRQ1 → INPUT_RING → sexinput)
- SexOS has NO PS/2 mouse driver (IRQ12 ignored)
- Even if SDL mouse routes to PS/2, SexOS cannot consume it

**All live pointer producer paths and their status:**

| Producer | Path | Status |
|---|---|---|
| USB tablet via xHCI | `sexusb → sexinput → silk-shell EV_ABS` | BLOCKED: no Transfer Events |
| USB mouse via xHCI | `sexusb → sexinput → silk-shell EV_REL` | BLOCKED: same |
| QMP/HMP injection | `qmp mouse_move → PS/2 absolute → i8042` | BLOCKED: PS/2 mouse not handled |
| Synthetic proof lane | `sexinput synthetic drag/click proof` | SKIP: proof env vars not set |
| Keyboard cursor fallback | `arrow keys → EV_REL via sexinput` | NOT BUILT: needs `SEXOS_KEYBOARD_CURSOR=1` |
| PS/2 mouse | IRQ12 | NOT IMPLEMENTED in SexOS |

---

## D) RECOMMENDED FIX

### Fix A — Smallest, zero code change (RECOMMENDED)

Use `SEXUSB_QEMU_DEVICE=tablet-display-sdl`. Already wired in dev.sh:

```bash
tablet-display-sdl) USB_DEVICE_ARG="-device usb-tablet,bus=xhci.0,display=sdl" ;;
```

Run command:
```bash
SEXOS_QEMU_DISPLAY=sdl SEXUSB_QEMU_DEVICE=tablet-display-sdl ./dev.sh 2>&1 | tee /tmp/cursor_live_tablet_sdl.log
```

**Why it works:** QEMU `display=sdl` parameter on usb-tablet binds the SDL window
mouse events directly to the USB tablet device. Moving mouse in QEMU window → QEMU
generates real USB HID absolute position report → xHCI Transfer Event → sexusb
`decode_tablet_report` (5-byte: buttons + ABS X u16 + ABS Y u16, range 0..32767) →
sexinput normalizer → silk-shell `EV_ABS` → `apply_abs_pointer` → `send_cursor_checked`
→ `cursor.motion.bounds ok=1`.

**Expected new markers (not seen before):**
```
[sexusb.xhci.event.transfer.intr_in] ...
[usb.mouse.report.raw] ...
[usb.mouse.to_normalizer] ...
[silk-shell.pointer.recv] class=EV_ABS ...
[cursor.motion.bounds] source=abs ... ok=1
[sexdisplay.cursor.visual.contrast] x=<new> y=<new> ... ok=1
```

**Verify after run:**
```bash
rg -n "cursor.motion.bounds|sexdisplay.cursor.draw|sexdisplay.cursor.visual.contrast|usb.mouse.report.raw|usb.mouse.to_normalizer|sexusb.xhci.event.transfer.intr_in|silk-shell.pointer.recv" \
  /tmp/cursor_live_tablet_sdl.log | tail -120
```

---

### Fix B — evdev passthrough (for headless/CI builds, no SDL needed)

```bash
./scripts/usb_pointer_real_report_operator_probe.sh evdev /tmp/usb_ptr_evdev.log
```

Requires: read access to `/dev/input/eventX` (be in `input` group, or use sudo).
Move mouse during the 45s probe window.

---

### Fix C — Keyboard cursor fallback (no SDL interaction needed)

Rebuild with `SEXOS_KEYBOARD_CURSOR=1`. Arrow keys and WASD emit EV_REL in sexinput.

```bash
SEXOS_KEYBOARD_CURSOR=1 ./scripts/entrypoint_build.sh
SEXOS_QEMU_DISPLAY=sdl SEXUSB_QEMU_DEVICE=tablet ./dev.sh
```

Good for cursor visual proof without physical mouse. Arrow keys move cursor visually.

---

### Fix D — Disable PS/2 to force USB routing (not recommended alone)

```bash
SEXOS_QEMU_I8042=off SEXOS_QEMU_DISPLAY=sdl SEXUSB_QEMU_DEVICE=tablet ./dev.sh
```

Disables i8042 PS/2 controller. May force QEMU to route keyboard input to USB HID too,
which can break keyboard if SexOS USB keyboard path isn't active. Not recommended without
also using `tablet-display-sdl`.

---

## E) FILES CHANGED

None. Audit only.

---

## F) NEXT PROMPT NAME

`LIVE_CURSOR_TABLET_SDL_PROOF_V1`

**Mission**: Run `SEXUSB_QEMU_DEVICE=tablet-display-sdl ./dev.sh`, move mouse in QEMU
window, verify `cursor.motion.bounds ok=1` and `sexdisplay.cursor.visual.contrast ok=1`
appear in the log. Gate: `cursor_visual_contrast PASS` + `cursor_motion_bounds PASS`.
