# QEMU_USB_KEYBOARD_MANUAL_PROOF_V1

**Status:** PASS DAILY + STRUCTURAL — manual visual confirmation deferred to human operator.
**Date:** 2026-05-16

---

## 1. Build: PASS — `[SEXOS ENTRYPOINT] success`
## 2. Daily Proof: 101/101 PASS, 0 faults

## 3. QEMU USB Boot

| Metric | Value |
|--------|-------|
| Lines | 8481 |
| Clock ticks | 39 |
| Faults | **0** |
| XHCI init | `[sexusb.xhci.map.ok]`, `[sexusb.xhci.probe.ok]` ✅ |
| USB keyboard device | `-device usb-kbd,bus=xhci.0` attached |
| USB mouse device | `-device usb-mouse,bus=xhci.0` attached |
| Golden hash | MATCH (0xFD6093AC9ADE7B4D) |
| Quil text render | 6 lines, 240 bytes visible (demo text + synthetic proof) |

## 4. Manual Visual Result: **DEFERRED** — requires human operator

Structural path confirmed: XHCI init succeeds, USB keyboard attaches, dispatch path wired (`pdx_call(SLOT_QUIL, OP_HID_EVENT)`). Manual typing test requires a human to:
1. Click on Quil surface to focus
2. Type "sex" on keyboard
3. Observe visible glyphs appear

## 5. Marker Summary
XHCI: OK. Synthetic typing: OK. Quil text recv/draw: OK. Faults: 0. Hash: match.

## 6. Fault Count: **0**

## 7. Remaining Gaps
- Manual visual confirmation (human operator needed)
- Physical USB hardware (not tested in QEMU)

## 8. Commit
```bash
git add docs/handoff/QEMU_USB_KEYBOARD_MANUAL_PROOF_V1.md
git commit -m "docs(input): QEMU USB keyboard manual proof V1"
```
