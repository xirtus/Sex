# SEXINPUT_TO_SHELL_ROUTE_AUDIT_V1

## Status

**PIPELINE PROVEN. No patch needed. (2026-05-04)**

---

## Actual Data Flow (confirmed)

```
sexusb  → OP_USB_MOUSE_REPORT (0x260) → sexinput
sexinput  → decode → normalize → EV_REL via OP_HID_EVENT (0x202) → silk-shell
silk-shell  → POINTER_X/Y update → OP_SURFACE_UPDATE → sexdisplay
sexdisplay  → cursor.surface.update → cursor.draw
```

**NOT** the direct `OP_USB_MOUSE_REPORT → shell` path. That path exists in shell but is only
used for synthetic click-focus proof (button state, not cursor movement). Cursor movement
goes through sexinput's normalizer → EV_REL.

---

## Proof Counts (`/tmp/synthetic-proof.log`, 122-frame synthetic run)

| Marker | Count | Notes |
|--------|-------|-------|
| `[sexinput.usb_mouse.recv]` | 122 | All frames received |
| `[sexinput.usb_mouse.normalize.ok]` | 122 | All normalized |
| `[sexinput.usb_mouse.shell_send.ok]` | 122 | All forwarded to shell |
| `[shell.cursor_surface.move.ok]` | 120 | Shell → sexdisplay surface update |
| `[sexdisplay.cursor.surface.update]` | 16 | Display render (batched) |
| `[sexdisplay.cursor.draw]` | 16 | Cursor drawn with changing x/y |
| `[sexdisplay.cursor_surface.z_top.ok]` | 1 | Cursor surface at z-top |
| `[sexdisplay.cursor_shape.arrow.ok]` | 1 | Arrow shape confirmed |
| panic / #PF / #GP | **0** | Clean |

Cursor x/y confirmed changing (e.g. `x=643 y=362` → `x=688 y=392` in drift phase).

16 draws vs 120 shell moves: sexdisplay renders at frame rate, not per-input-event. Normal.

---

## Why Original Mission Markers Were Zero

The mission brief searched for stale marker names:

| Mission Marker | Status | Real Equivalent |
|----------------|--------|-----------------|
| `shell.pointer.usb_state.ok` | stale — only fires on OP_USB_MOUSE_REPORT direct path | `[silk-shell] Pointer REL d=(...) pos=(N,N)` |
| `sexdisplay.cursor_state.recv` | stale — not emitted in current code | `[sexdisplay.cursor.surface.update]` |

The pipeline was working the whole time. Marker names in mission brief were outdated.

---

## Architecture Note

`silk-shell` has **two** mouse-related handlers:

1. **OP_USB_MOUSE_REPORT (0x260)** — used by synthetic click-focus proof for button edge events.
   Explicitly does NOT update POINTER_X/Y from dx/dy (comment at line ~626: "eliminates dx/dy double-apply bug").

2. **OP_HID_EVENT (0x202) / EV_REL** — the real cursor movement path. sexinput normalizer
   emits this for every relative movement event. Shell updates POINTER_X/Y here.

---

## Build/Run Reference

```bash
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>&1 | tee /tmp/synthetic-proof.log
```

## Next

Physical input retest options documented in SYNTHETIC_INPUT_PROOF_V1.md.
