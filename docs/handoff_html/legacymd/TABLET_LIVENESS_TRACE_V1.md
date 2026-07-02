# TABLET_LIVENESS_TRACE_V1

## Mission Objective
Diagnose why cursor input stops ~7s after boot while system clock keeps running.
Bounded periodic markers at each pipeline stage, max 16 per marker type.

## Patched Files
- `servers/sexusb/src/main.rs` — added `[sexusb.tablet.live]` marker
- `servers/sexinput/src/main.rs` — added `[sexinput.mouse.live]`, `[sexinput.hid.emit.rel]`
- `servers/silk-shell/src/main.rs` — added `[shell.hid.rel.live]`, `[shell.cursor.surface.update]`
- `servers/sexdisplay/src/main.rs` — added `[sexdisplay.cursor.surface.update]`, `[sexdisplay.cursor.draw]`

## Marker Pipeline
```
sexusb —[OP_USB_MOUSE_REPORT]→ sexinput —[OP_HID_EVENT, EV_REL]→ silk-shell —[OP_SURFACE_UPDATE]→ sexdisplay
  [sexusb.tablet.live]           [sexinput.mouse.live]               [shell.hid.rel.live]             [sexdisplay.cursor.surface.update]
  [sexusb.forward.mouse]         [sexinput.hid.emit.rel]             [shell.cursor.surface.update]    [sexdisplay.cursor.draw]
```

## Test Command
```
SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
SEXUSB_QEMU_DEVICE=tablet SEXOS_QEMU_DISPLAY=sdl-grab timeout 20 ./dev.sh run \
  2>/tmp/tablet-live.trace | tee /tmp/tablet-live.log
```
During visual run: move trackpad continuously from 5s to 15s. Do not click.

## Marker Counts (non-interactive run, no mouse movement)
```
sexusb.tablet.live:             0  (only fires on non-idle reports with movement)
sexusb.forward.mouse:          15  (tablet reports forwarded to sexinput)
sexinput.mouse.live:           15  (sexinput received raw mouse reports)
sexinput.hid.emit.rel:          0  (normalizer did not emit EV_REL — dx=dy=0)
shell.hid.rel.live:             0  (shell received no EV_REL)
shell.cursor.surface.update:    0  (shell sent no cursor update via EV_REL)
sexdisplay.cursor.surface.update: 0  (display received no cursor update)
sexdisplay.cursor.draw:         8  (cursor drawn 8× at 640,360 during startup)
panic|PAGE FAULT|GENERAL PROTECTION: 0
```

## Last Marker Values
```
sexusb.forward.mouse:          buttons=0x0 packed=0x0
sexinput.mouse.live:           dx=0 dy=0 buttons=0x0
sexinput.mouse.real.delta:     dx=0 dy=0 buttons=0x0
sexdisplay.cursor.draw:        x=640 y=360
```

## Analysis
1. **sexusb → sexinput pipeline is alive** — 15 OP_USB_MOUSE_REPORT forwarded.
2. **All reports have packed=0x0** — meaning dx=0, dy=0, buttons=0 for every report in this non-interactive run. No host mouse movement reached the QEMU guest.
3. **sexinput normalizer correctly suppresses EV_REL** when dx=dy=0 — thus 0 `hid.emit.rel`.
4. **sexdisplay.draw cursor at (640, 360)** — initial center position from startup renders.
5. **Synthetic proofs disabled** — `proof.gate.state enabled=0 source=env`. No synthetic drag/click traffic in the pipeline.
6. **No panics, page faults, or protection faults.**
7. **Clock keeps counting** — independent of input path; powered by local fallback clock in sexdisplay.

## Root Cause
**Cannot determine without interactive mouse movement.** The tablet produces absolute coordinates (abs_x, abs_y). The delta computation requires coordinate changes, which only occur when the host mouse moves inside the QEMU SDL window. All markers show healthy pipeline semantics for the zero-movement case.

## How to Complete Diagnosis (Interactive Test)
1. Run with SDL window visible: `SEXUSB_QEMU_DEVICE=tablet SEXOS_QEMU_DISPLAY=sdl-grab timeout 20 ./dev.sh run`
2. Move trackpad continuously from 5s to 15s inside the QEMU window
3. Check if `[sexusb.tablet.live]` fires — if no, tablet IRQ/transfer path is broken
4. Check if `[sexinput.hid.emit.rel]` fires — if no, normalizer not producing EV_REL
5. Check if `[shell.hid.rel.live]` fires — if no, shell not receiving HID events
6. Follow downstream from there

## Files Touched
- `docs/handoff/TABLET_LIVENESS_TRACE_V1.md` (this file)
- `servers/sexusb/src/main.rs` (marker only)
- `servers/sexinput/src/main.rs` (marker only)
- `servers/silk-shell/src/main.rs` (marker only)
- `servers/sexdisplay/src/main.rs` (marker only)
- `CLAUDE.md` (update)

## Forbidden Changes NOT Made
- No kernel/ changes
- No crates/sex-pdx/ changes
- No PDX ABI changes
- No renderer rewrite
- No tiling/glass/window manager work
- No shared-buffer redesign
- No proof code removal
