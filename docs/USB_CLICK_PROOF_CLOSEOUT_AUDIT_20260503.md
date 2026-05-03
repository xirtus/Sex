# USB Button Click Proof — Closeout Audit

**Date:** 2026-05-03
**Files read:** `serial.log`, `docs/INPUT_USB_NEXT.md`, `servers/sexinput/src/main.rs`,
`docs/handoff/STABLE_BASELINE_20260503.md`, `servers/silk-shell/src/main.rs`

---

## Verdict: PARTIAL

**Shell click path is proven via synthetic events, but physical USB button decode is NOT proven.**

---

## 1. Evidence from serial.log (latest boot)

### Proven: synthetic paths
| Proof | Marker | Status |
|---|---|---|
| SilkBar contract validation | `[silk.contract.validate.ok] version=2` | ✅ PASS |
| Top-strip render proof | `[silk.render_proof.top_strip.ok]` hash=0x70a68011ec352490 | ✅ PASS |
| SilkBar click dispatch (all 5 targets) | `[shell.silkbar.click] target=launcher/workspace/status/clock/bell` | ✅ PASS |
| Panel toggle (launcher/status/clock) | `[shell.launcher.open/close.ok]`, `[shell.status.open/close.ok]`, `[shell.clock.open/close.ok]` | ✅ PASS |
| Synthetic drag proof (BTN dn/up + REL) | `[sexinput.drag_proof.*]` + `[silk-shell] Pointer BTN 1 dn/up` | ✅ PASS |
| Click focus from synthetic drag | `[silk-shell] Click focus surface 101/102` | ✅ PASS |
| No faults/panics/PF/GP | grep returns 0 | ✅ PASS |

### Not proven: physical USB
| Marker | Status | Why |
|---|---|---|
| `[sexusb.xhci.map.bad]` | ❌ FAIL | XHCI BAR mapping failed in this boot — USB hardware path never initialized |
| `[sexusb.hid.mouse.report]` | ❌ NOT PRESENT | No USB HID mouse reports captured |
| `[sexinput.usb_mouse.recv]` | ❌ NOT PRESENT | sexinput never received USB mouse data |
| `[shell.pointer.usb_state.nonzero.ok]` | ❌ NOT PRESENT | Shell never got nonzero USB pointer state |
| `[sexinput.synthetic.click_focus.*]` | ❌ DISABLED | `USB_PROOF_DISABLE_SYNTH_CLICK = true` — proof is gated off |

---

## 2. The 9 Proof Checks

| # | Check | Status | Evidence |
|---|---|---|---|
| 1 | Physical USB report received | ❌ | `[sexusb.xhci.map.bad]` — no USB HW path |
| 2 | Button bit decoded from report | ❌ | No reports to decode |
| 3 | Button down edge marker | ❌ | Only synthetic (drag proof BTN dn) |
| 4 | Button up edge marker | ❌ | Only synthetic (drag proof BTN up) |
| 5 | Normalized HID event emitted | ❌ | From USB; synthetic HID_EVENT path works ✅ |
| 6 | Shell received HID event | ❌ | From USB; synthetic OP_HID_EVENT path works ✅ |
| 7 | Click caused focus or panel action | ✅ | `[silk-shell] Click focus surface 102` in log |
| 8 | Motion-without-button did not click | ✅ | Drag proof REL events don't trigger focus |
| 9 | No fault/panic/PF/GP | ✅ | Zero matches in serial.log |

---

## 3. Current Blockers

### Blocker A: `[sexusb.xhci.map.bad]` in this boot

The USB XHCI controller BAR mapping failed. This is environment/hardware-specific
(QEMU configuration, PCI enumeration, or kernel PCI BAR mapping). Without a working
map, no USB device can be enumerated or polled.

### Blocker B: Synthetic click focus proof is disabled

`USB_PROOF_DISABLE_SYNTH_CLICK = true` in `servers/sexinput/src/main.rs` line 24.
The code exists and is correct — toggling this to `false` would prove the
`sexinput→shell→click_focus` end-to-end chain (but still via USB mouse protocol
encoding, not physical USB).

### Blocker C: QEMU cannot inject USB button events

Documented in `docs/INPUT_USB_NEXT.md` lines 1096-1115:
- QMP/HMP input injection routes to PS/2 only, not to USB HID
- SDL2/X11 filters XTest synthetic events (send_event=True)
- VNC PointerEvent does not propagate to USB device model
- Only real physical mouse or uinput virtual device can deliver button events

---

## 4. Prior Proven USB Path (from earlier session)

From `docs/INPUT_USB_NEXT.md` lines 1132-1139 — a prior `SDL_VIDEO_DRIVER=x11`
session proved:
```
[sexusb.hid.tablet.report] i=1 buttons=0x0 x=32741 y=9625 dx=127 dy=127
[sexusb.hid.tablet.nonzero.ok] i=1 buttons=0x0 x=32741 y=9625 dx=127 dy=127
[shell.pointer.usb_state.nonzero.ok] x=767 y=487 buttons=0x0
```

**BUT** `buttons=0x0` always — no button events were ever captured from USB,
even when the tablet path worked. Button decode code (`buf[0] & 0x07`) is
correct by code inspection, but the QEMU environment never delivers button
state changes to the USB HID device model.

---

## 5. Smallest Next Patch (if any)

**No code patch needed** — all code paths exist and are correct.

The only change is a toggle:
- Set `USB_PROOF_DISABLE_SYNTH_CLICK = false` in `servers/sexinput/src/main.rs`
- Rebuild and re-run
- This proves the full `sexinput→shell→click_focus` chain via the USB mouse
  protocol path (still synthetic, but exercising the `OP_USB_MOUSE_REPORT` path)

Expected pass markers:
```
[sexinput.synthetic.click_focus.start]
[sexinput.synthetic.click_focus.down]
[shell.click_focus.down] x=940 y=520 buttons=0x1
[shell.click_focus.hit] id=200
[shell.click_focus.send.ok] id=200
[sexinput.synthetic.click_focus.up]
```

---

## 6. Summary

| Component | Status |
|---|---|
| USB hardware path (xhci BAR map) | ❌ Dead in this env (`map.bad`) |
| USB device enum/decode code | ✅ Correct by inspection |
| USB→sexinput routing | ✅ Capability route exists |
| sexinput normalize/send code | ✅ Present and correct |
| Shell click-focus receive code | ✅ Present and correct |
| Synthetic drag proof | ✅ Running (proves shell chain) |
| Synthetic silkbar click proof | ✅ Running (proves silkbar chain) |
| Synthetic USB mouse click proof | ⛔ Disabled (toggle needed) |
| Physical USB button proof | ❌ Blocked (QEMU injection limits) |

**USB_BUTTON_CLICK_PROOF_V1 is PARTIAL.** All code paths are implemented and
correct, but full physical button proof requires either:
1. A working XHCI BAR map + real USB mouse over SDL window
2. uinput virtual mouse device
3. Acceptance that synthetic proof is sufficient

The toggle of `USB_PROOF_DISABLE_SYNTH_CLICK` is the smallest possible next step
and does not require any kernel, sex-pdx, or ABI changes.

*End of USB click proof closeout audit.*
