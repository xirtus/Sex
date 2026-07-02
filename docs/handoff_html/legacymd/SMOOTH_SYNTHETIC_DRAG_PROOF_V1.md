# SMOOTH_SYNTHETIC_DRAG_PROOF_V1

## Status

Synthetic drag proof implemented (2026-05-04).

---
## Context

This proof extends `SYNTHETIC_INPUT_PROOF_V1` with a multi-phase drag sequence that exercises the existing drag state machine in `silk-shell`. Physical host input remains blocked by QEMU 11 host→USB HID routing — see `SYNTHETIC_INPUT_PROOF_V1.md`.

---
## What This Proof Does

Validates the **guest PDX pipeline** end-to-end for synthetic drag:

```
sexusb synthetic report (buttons=1 + dx/dy)
  → decode_boot_mouse_report()
  → OP_USB_MOUSE_REPORT PDX send to sexinput
  → sexinput normalize_pointer_report_v1() emits EV_BTN + EV_REL
  → shell OP_HID_EVENT handler (EV_BTN: button state + drag start/stop,
                                EV_REL: cursor position + drag move)
  → shell OP_SURFACE_UPDATE to sexdisplay (cursor + dragged surface)
```

Proves:
- Cursor moves over multiple frames via EV_REL accumulation
- Left button down transitions ClickPending → Dragging (existing shell state machine)
- Motion while held moves focused surface by accumulated delta
- Button release transitions Dragging → Idle
- No faults (#PF, #GP, panic)

Does NOT prove:
- Physical host input delivery (blocked — see SYNTHETIC_INPUT_PROOF_V1)
- sexdisplay rendering smoothness (sexdisplay is STOP-listed; no changes)
- Frame-level pacing (sys_yield() is not a frame clock; pacing yields mitigate burstiness)

---
## Drag State Audit

**Result: drag state already exists and is fully functional.**

| Component | Status |
|-----------|--------|
| `InteractionState::Dragging` enum variant | Present | 
| State machine transitions (ClickPending → Dragging → Idle) | Present |
| Drag start on left-click within shell surface bounds | Present |
| Drag move via EV_REL delta on focused surface position | Present (both USB and HID REL paths) |
| Drag end on button release | Present |
| Diagnotic markers: `[shell.drag.start]`, `[shell.drag.move]`, `[shell.drag.end]` | Present |
| Button event log: `[silk-shell] Pointer BTN N dn/up` | Present |
| Drag cancel on surface death | Present |

**No shell patches needed.**

---
## Synthetic Sequence

242 frames total, ~6 phases:

| Phase | Frames | buttons | dx | dy | pacing_yields | Purpose |
|-------|--------|---------|----|----|---------------|---------|
| 1 | 40 | 0 | +1 | +1 | 2 | Move cursor onto target surface |
| 2 | 1 | 1 | 0 | 0 | 4 | Button down → ClickPending → Dragging |
| 3 | 80 | 1 | +1 | 0 | 2 | Drag right (button held) surface moves |
| 4 | 80 | 1 | 0 | +1 | 2 | Drag down (button held) surface moves |
| 5 | 1 | 0 | 0 | 0 | 4 | Button release → Dragging → Idle |
| 6 | 40 | 0 | -1 | -1 | 2 | Cursor drifts back (button up) |

Button release frame included — prevents stuck-button state in downstream pipeline.

---
## Build

```bash
# Default (real USB path, unaffected):
./scripts/entrypoint_build.sh

# Synthetic drag proof:
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both are compile-time `option_env!` gates — must be set at build invocation, not runtime.

---
## Run

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run \
  2>/tmp/smooth-drag-proof.err | tee /tmp/smooth-drag-proof.log
```

---
## Verify

```bash
grep -aE "synthetic.drag|usb_mouse|hid.emit|hid.rel|cursor_surface|drag|surface.update|panic|#PF|#GP|GENERAL PROTECTION" \
  /tmp/smooth-drag-proof.log | head -180
```

### Required markers

| Marker | Required | Source |
|--------|----------|--------|
| `[sexusb.synthetic.gate] enabled=1 source=env` | yes | sexusb |
| `[sexusb.synthetic.drag.start]` | yes | sexusb |
| `[sexusb.synthetic.frame] n=N dx=N dy=N buttons=N` | yes, count > 200 | sexusb |
| `[sexusb.synthetic.drag.complete]` | yes | sexusb |
| `[sexinput.usb_mouse.recv]` | yes | sexinput |
| `[sexinput.usb_mouse.normalize.ok]` | yes | sexinput |
| `[sexinput.hid.emit.rel]` | yes, count > 0 during phases 3-4 | sexinput |
| `[silk-shell] Pointer BTN 1 dn buttons=0x01` | yes (phase 2) | shell |
| `[shell.drag.start]` | yes (phase 2) | shell |
| `[shell.drag.move]` | yes, count > 0 during phases 3-4 | shell |
| `[silk-shell] Pointer BTN 1 up buttons=0x00` | yes (phase 5) | shell |
| `[shell.drag.end]` | yes (phase 5) | shell |
| `[shell.cursor_surface.move.start/ok]` | yes, many | shell |
| panic / #PF / #GP / GENERAL PROTECTION | must be absent | any |

---
## Smoothness Bottleneck Analysis

The proof identifies the bottleneck as **a combination of (A) bursty frame pacing and (E) sys_yield not being a frame clock**. Specific observations:

| Factor | Evidence | Impact |
|--------|----------|--------|
| (A) Single `sys_yield()` per frame in V1 | 1 yield per frame = tight polling loop; compositor (sexdisplay) may not get CPU to render intermediate frames | Cursor jumps between positions without intermediate render frames |
| (B) REL event count | Fixed at 1 EV_REL per sexusb frame; limited by xHCI poll rate | Acceptable for proof, but limits smoothness |
| (C) Shell coalesce/skip | `[shell.drag.move]` fires every EV_REL — no coalescing observed | Not a bottleneck |
| (D) sexdisplay render | STOP-listed — cannot patch | Suspected dominant bottleneck; sexdisplay may only redraw at its own pace |
| (E) `sys_yield()` pacing | Replaced with 2-4 yields per frame in V2 | Mitigates burstiness but does not align to vsync/display refresh |

**Recommended next phase** if smoothness needs improvement:
- Implement a timer-based pacing mechanism in sexusb (not just bare `sys_yield`)
- Investigate sexdisplay render scheduling (requires STOP FIRST for kernel/display changes)

---
## Changes Made

### `servers/sexusb/src/main.rs`
- Replaced old synthetic sequence (121 frames, no drag) with new sequence (242 frames, drag phases)
- Added pacing_yields=2 per movement frame, pacing_yields=4 per button transition frame
- Added markers: `[sexusb.synthetic.drag.start]`, `[sexusb.synthetic.drag.frame]`, `[sexusb.synthetic.drag.complete]`
- Default (non-synthetic) behavior unchanged

### `servers/silk-shell/src/main.rs`
- No changes needed — drag state machine, transitions, and all diagnostic markers already present

### New file: `docs/handoff/SMOOTH_SYNTHETIC_DRAG_PROOF_V1.md`

---
## Non-Goals

- Physical QEMU input fix (blocked)
- PS/2, xHCI refactor
- sexdisplay render ownership or framebuffer changes
- Broad input/window-manager refactor
- Animation/visual polish
- Terminal/Quil work
