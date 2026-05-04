# SYNTHETIC_INPUT_PROOF_V1

## Status

Synthetic proof gate implemented and both builds confirmed passing (2026-05-04).

---

## Physical Host Input — BLOCKED

QEMU 11.0.0 host→USB HID routing is broken on this host. All delivery paths tried:

| Method | Result |
|--------|--------|
| i8042=off | zero bytes in USB HID reports |
| SDL display | zero |
| GTK display | zero |
| VNC display | zero |
| QMP `input-send-event` | zero |
| HMP `sendkey` | zero |
| Physical tablet/mouse | xHCI ring receives reports, dx=0 dy=0 buttons=0 |

**Conclusion:** Physical host input cannot be delivered to USB HID reports in QEMU 11 on this host. Stop chasing it. This is a host/QEMU version issue, not a guest pipeline bug.

---

## What This Proof Does

Validates the **guest PDX pipeline only**:

```
sexusb synthetic report
  → decode_boot_mouse_report()
  → OP_USB_MOUSE_REPORT PDX send
  → sexinput USB mouse recv + normalize
  → shell pointer/cursor state update
  → sexdisplay cursor surface receive + draw
```

Does NOT prove physical host input delivery.

---

## Gate

`servers/sexusb/src/main.rs`:
```rust
const SEXUSB_SYNTHETIC: bool = option_env!("SEXUSB_SYNTHETIC").is_some();
```

- Default (unset): real xHCI interrupt-IN poll loop runs unchanged.
- Set at build time: synthetic sequence runs instead, then parks. Real poll loop unreachable.

---

## Synthetic Sequence

121 frames total:

| Phase | Frames | buttons | dx | dy |
|-------|--------|---------|----|----|
| Drift right/down | 60 | 0 | +3 | +2 |
| Click press | 1 | 1 | 0 | 0 |
| Click release | 1 | 0 | 0 | 0 |
| Drift left/down | 60 | 0 | -2 | +1 |

Button release frame included — prevents stuck-button state in downstream pipeline tests.

---

## Build

```bash
# Default (real USB path, unaffected):
./scripts/entrypoint_build.sh

# Synthetic proof:
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

`SEXOS_PROOFS_DISABLED=1` disables sexinput proof gates for interactive use.
Both are compile-time `option_env!` gates — must be set at build invocation, not runtime.

---

## Run

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run \
  2>/tmp/synthetic-proof.err | tee /tmp/synthetic-proof.log
```

---

## Verify

```bash
grep -aE "synthetic|usb_mouse|normalize|shell_send|pointer.usb_state|cursor_state|sexdisplay.cursor" \
  /tmp/synthetic-proof.log | head -120
```

### Required markers

| Marker | Required |
|--------|----------|
| `[sexusb.synthetic.gate] enabled=1 source=env` | yes |
| `[sexusb.synthetic.start]` | yes |
| `[sexusb.synthetic.frame] n=0..N` | yes (dx/dy/buttons vary) |
| `[sexusb.synthetic.send.ok]` | count > 0 |
| `[sexinput.usb_mouse.recv]` | yes |
| `[sexinput.usb_mouse.normalize.ok]` | yes |
| `[shell.pointer.usb_state.ok] x=N y=N` | yes, x/y changing |
| `[sexdisplay.cursor_state.recv]` | yes |
| panic / #PF / #GP | must be absent |

---

## Future Physical Input Retest Options

1. **Different QEMU version** — QEMU 9.x or earlier may have working host→USB HID routing.
2. **GTK + qemu-xhci swap** — one-line change in `dev.sh`. STOP if requires xHCI refactor.
3. **usbredir / VFIO** — pass physical USB device through directly.
4. **Hardware boot** — bypass QEMU entirely, boot from USB stick.
