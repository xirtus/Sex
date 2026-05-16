# INPUT_REAL_DEVICE_RELIABILITY_AUDIT_V1

**Status:** PASS REVIEW ONLY.
**Date:** 2026-05-16
**Depends on:** `QUIL_VISIBLE_TYPING_E2E_V1.md`.

---

## 0. PASS REVIEW ONLY — Synthetic path proven, physical USB needs operator test

---

## 1. Reliability Truth Table

| Input Source | Synthetic | QEMU USB | Physical USB | Notes |
|-------------|-----------|----------|-------------|-------|
| Keyboard typing | ✅ Proven (Quil E2E) | ⚠️ Same dispatch, needs visual confirm | ❌ Not proven (no USB HID decode proof) | Same `pdx_call(SLOT_QUIL, OP_HID_EVENT)` path |
| Mouse movement | ✅ Synthetic drag proof | ⚠️ Needs verify | ❌ Not proven | sexinput normalizes reports |
| Mouse click/focus | ✅ Synthetic click focus | ⚠️ Needs verify | ❌ Not proven | Shell click→focus path proven |
| Pointer tracking | ✅ Shell pointer state | ⚠️ Needs verify | ❌ Not proven | SilkBar click targets work |
| Drag | ✅ Synthetic drag proof | ❌ Not tested | ❌ Not proven | |
| Trackpad | ❌ Not implemented | ❌ N/A | ❌ Not implemented | |
| Long-run stability | ✅ 30s daily proof | ✅ Clean 18s smokes | ❌ Not tested beyond 30s | |

---

## 2. Known Issues

| Issue | Status |
|-------|--------|
| XHCI BAR mapping fails in some boots | `[sexusb.xhci.map.bad]` — USB hardware path never initializes |
| USB boot delay | XHCI init adds seconds; normal for emulated QEMU |
| Keyboard freeze (10-17 sec) | Not documented in current codebase; may be XHCI init window |
| `[sexinput.pointer.drop]` | Shell send fail — non-fatal, proof marker only |
| `[sexinput.usb_kbd.drop]` | Unmapped HID usage code — non-fatal |

---

## 3. Recommended Next Target: **A — QEMU_USB_KEYBOARD_MANUAL_PROOF_V1**

Simple manual operator test — no code changes needed.

### Manual Test Recipe

```bash
# 1. Build
./scripts/entrypoint_build.sh

# 2. Boot QEMU with USB keyboard
qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku \
  -cdrom ./sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci \
  -device usb-kbd,bus=xhci.0 \
  -device usb-mouse,bus=xhci.0 \
  -serial file:/tmp/manual_kbd_test.log \
  -display gtk -boot d

# 3. In QEMU GUI:
#    - Click on Quil surface to focus
#    - Type "sex" on keyboard
#    - Observe visible characters appear in Quil surface

# 4. Verify markers
grep "quil.text.recv" /tmp/manual_kbd_test.log
grep "#PF\|#GP\|fault.kill\|KERNEL PANIC" /tmp/manual_kbd_test.log

# 5. Pass criteria:
#    - [quil.text.recv] markers present for typed chars
#    - 0 faults
#    - Visible glyphs in QEMU window
```

---

## 4. Blockers

| Blocker | For What |
|---------|----------|
| No physical USB keyboard device available in CI | Physical USB path can only be proven manually |
| XHCI init may fail in some boots | QEMU USB device may not enumerate |
| `sexusb` may not be running | Check `[kernel.spawn.sexusb]` in log |

---

## 5. STOP FIRST Boundaries (all pass)

| Boundary | Status |
|----------|--------|
| Kernel/ABI edit | ❌ No |
| USB stack rewrite | ❌ No |
| Input protocol redesign | ❌ No |
| Renderer changes | ❌ No |
| Broad shell refactor | ❌ No |

---

## 6. Handoff

```
docs/handoff/INPUT_REAL_DEVICE_RELIABILITY_AUDIT_V1.md
```

## 7. Commit

```bash
git add docs/handoff/INPUT_REAL_DEVICE_RELIABILITY_AUDIT_V1.md
git commit -m "docs(audit): input real device reliability V1"
```
