# USB_MOUSE_REL_DECODE_FIX_V1

Date: 2026-05-14

## 1) Root cause
The observed `[shell.abs.normalize] raw_x=200 raw_y=200` in usb-mouse runs was caused by default synthetic pointer proof injections in `sexinput` (`EV_ABS` anchor events), not by boot-mouse REL decode itself.

Specifically:
- `sexusb` forwards boot mouse deltas packed in `OP_USB_MOUSE_REPORT`.
- `sexinput` decodes those as REL (`is_abs=0`) and emits HID REL correctly.
- In parallel, when proofs are enabled (default), `sexinput` also injects synthetic ABS events (`200,200` and click-focus ABS anchors), which made shell logs appear as if mouse traffic was ABS.

## 2) Fix applied (minimal, no ABI/opcode/routing change)
- Added runtime latch in `sexinput`: `REAL_USB_POINTER_SEEN`.
- Set latch on first real `OP_USB_MOUSE_REPORT` receive.
- Suppressed synthetic pointer ABS proof injectors once real USB pointer input is present:
  - synthetic drag proof block
  - synthetic click-focus proof block
- Added diagnostics:
  - `[sexinput.pointer.mode] mode=rel|abs|btn ...`
  - `[silk-shell.rel.recv] dx=N dy=N buttons=N`

This keeps real usb-mouse movement on REL path and removes synthetic ABS interference during mouse sessions.

## 3) Files changed
- `servers/sexinput/src/main.rs`
- `servers/silk-shell/src/main.rs`

## 4) Build result
`./scripts/entrypoint_build.sh` completed successfully.

## 5) Runtime proof commands
Use usb-mouse lane:

```bash
# build (already done)
./scripts/entrypoint_build.sh

# run your GTK boot lane with:
# -device nec-usb-xhci,id=xhci
# -device usb-mouse,bus=xhci.0

grep -E "sexusb.*mouse|sexinput.pointer.raw|sexinput.pointer.mode|silk-shell.rel.recv|shell.abs.normalize|shell.cursor.final.send|sexdisplay.cursor.draw|fault.kill|#PF|#GP|panic|KERNEL PANIC" "$LOG" | tail -1200
```

Expected:
- `sexinput.pointer.mode` shows `mode=rel` for real mouse movement.
- `silk-shell.rel.recv` appears with nonzero `dx/dy`.
- `shell.cursor.final.send source=rel` appears.
- No synthetic ABS `raw_x=200 raw_y=200` after real mouse stream starts.
- fault count remains 0.

## 6) Notes
Tablet ABS path was not altered.
