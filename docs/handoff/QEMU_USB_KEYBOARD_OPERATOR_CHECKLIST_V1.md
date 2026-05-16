# QEMU_USB_KEYBOARD_OPERATOR_CHECKLIST_V1

**Status:** Docs-only operator checklist. No source changes.
**Date:** 2026-05-16
**Depends on:** `QEMU_USB_KEYBOARD_MANUAL_PROOF_V1.md`.

---

## Operator Checklist

### Step 1: Build
```bash
./scripts/entrypoint_build.sh
# Expect: [SEXOS ENTRYPOINT] success
```

### Step 2: Boot QEMU with USB keyboard
```bash
LOG=/tmp/sexos_qemu_usb_keyboard_operator_check_v1.log
rm -f "$LOG"
qemu-system-x86_64 \
  -M q35 -m 512M -cpu max,+pku \
  -cdrom ./sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci \
  -device usb-kbd,bus=xhci.0 \
  -device usb-mouse,bus=xhci.0 \
  -serial file:"$LOG" \
  -display gtk -boot d
```

### Step 3: Wait for desktop
SilkBar at top, 4 visible surfaces (Spindle, Quil, Linen, Browser).
Confirm XHCI initialized: `grep "xhci.probe.ok" "$LOG"`

### Step 4: Focus Quil
- Click on the Quil surface (usually middle-left, labeled "Quil")
- Or use keyboard: Alt+2 (if launcher slot 2 = Quil)
- Confirm focus: Quil frame rim should brighten (focused intensity)

### Step 5: Type "sex"
Type the letters: `s` `e` `x`
Observe visible glyphs appear in the Quil surface.

### Step 6: Wait 30 seconds
Let the system run. Clock should tick. No crashes.

### Step 7: Close QEMU
Press Ctrl+C in terminal or close QEMU window.

### Step 8: Verify markers
```bash
echo "=== Faults ===" && grep -ci "#PF\|#GP\|fault.kill\|KERNEL PANIC" "$LOG"
echo "=== XHCI ===" && grep "xhci.probe" "$LOG"
echo "=== Quil text ===" && grep "quil.text.recv\|quil.text.draw" "$LOG" | tail -10
echo "=== Ticks ===" && grep -c "ticks=" "$LOG"
echo "=== Hash ===" && grep "topstrip.hash.result" "$LOG"
```

---

## PASS Criteria
- [ ] Visible "sex" or equivalent chars in Quil surface
- [ ] `[quil.text.recv]` markers present for typed chars
- [ ] `[quil.text.draw.v2]` marker confirms render
- [ ] 0 faults (#PF, #GP, fault.kill, KERNEL PANIC = 0)
- [ ] Clock kept ticking after typing (ticks > 40)
- [ ] Golden hash matches (0xFD6093AC9ADE7B4D)

---

## FAIL Actions

| Failure | Action |
|---------|--------|
| No focus on Quil | Try clicking Quil surface with mouse; check `[shell.focus.set]` marker |
| Keys reach shell but not Quil | Check `pdx_call(SLOT_QUIL, OP_HID_EVENT)` dispatch; verify Quil is listening |
| Quil receives but no render | Check `draw_text_lines()` is called; verify sexdisplay surface is alive |
| Render but faults later | Save log; check `#PF`/`#GP` markers; report to handoff |
| XHCI not attached | QEMU may need `-device nec-usb-xhci`; try different QEMU version |

---

## Result Template (paste back)

```
BUILD:    [  ] PASS  [  ] FAIL
XHCI:     [  ] OK    [  ] FAIL
FOCUS:    [  ] Quil focused
TYPED:    [  ] "sex" visible in Quil  [  ] not visible
FAULTS:   [  ] 0     [  ] ___ found
TICKS:    ___
HASH:     [  ] MATCH  [  ] MISMATCH
OPERATOR: ___
DATE:     ___
```

---

## Handoff
```
docs/handoff/QEMU_USB_KEYBOARD_OPERATOR_CHECKLIST_V1.md
```

## Commit
```bash
git add docs/handoff/QEMU_USB_KEYBOARD_OPERATOR_CHECKLIST_V1.md
git commit -m "docs(input): QEMU USB keyboard operator checklist V1"
```
