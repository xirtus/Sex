# USB_CURSOR_ROUTE_PROOF_V1

**Status:** Diagnostic complete. Root cause identified.
**Launcher:** `./qemuX.sh` — patched QEMU 9.2.0 (v9.2.0-dirty) at `tools/qemu/build/qemu-system-x86_64`
**Date:** 2026-05-05

---

## 1. Files Changed

| File | Change | Type |
|------|--------|------|
| `docs/handoff/USB_CURSOR_ROUTE_PROOF_V1.md` | This document | Handoff |

**No code changes.** All diagnostic markers already existed from prior INPUT_DELIVERY_TRACE_V1 work.

### Existing markers used for tracing

| Stage | Marker | Source |
|-------|--------|--------|
| QEMU HID event routing | `[QEMU_TRACE] Routing event type 3 to handler: QEMU HID Tablet` | QEMU guest debug |
| USB HID report (raw) | `[sexusb.hid.tablet.raw] b0=... actual=6` | sexusb/src/main.rs:2695 |
| USB HID decode | `[sexusb.hid.tablet.report]` / `[sexusb.hid.tablet.nonzero.ok]` | sexusb/src/main.rs:2763/2769 |
| sexusb → sexinput | `[sexusb.forward.mouse] buttons=0x... packed=0x...` | sexusb/src/main.rs:2734/2794 |
| sexinput delta | `[sexinput.mouse.real.delta] dx=... dy=... buttons=...` | sexinput/src/main.rs:187 |
| sexinput live | `[sexinput.mouse.live] n=... dx=... dy=... buttons=...` | sexinput/src/main.rs |
| shell cursor | `[shell.cursor.move] x=... y=...` | silk-shell/src/main.rs:9825/10773 |
| display draw | `[sexdisplay.cursor.draw] n=0 x=... y=...` | sexdisplay/src/main.rs:712 |

---

## 2. Boot Log — Full Chain Trace

```
1. QEMU:   [QEMU_TRACE] Routing event type 3 to handler: QEMU HID Tablet     (repeated many times)

2. sexusb: [sexusb.hid.tablet.raw] b0=0x0 b1=0x0 b2=0x0 b3=0x0 b4=0x0 actual=6

3. sexusb: [sexusb.forward.mouse] buttons=0x0 packed=0x0

4. sexinput: [sexinput.mouse.real.delta] dx=0 dy=0 buttons=0x0

5. sexinput: [sexinput.mouse.live] n=0 dx=0 dy=0 buttons=0x0

6. sexdisplay: [sexdisplay.cursor.draw] n=0 x=640 y=360                                             (cursor at center, never moves)
```

**No [sexusb.hid.tablet.nonzero.ok] or [shell.cursor.move] observed.** All numeric values are zero.

---

## 3. Stage-by-Stage Diagnosis

### Stage 1: QEMU HID event routing — ⚠️ PARTIAL

```
[QEMU_TRACE] Routing event type 3 to handler: QEMU HID Tablet
```

The patched QEMU **does** receive host input events and routes them to the `QEMU HID Tablet` handler. However, this routing does **not** result in non-zero HID report bytes in the USB interrupt-in buffer.

### Stage 2: USB HID report — ❌ BROKEN

```
[sexusb.hid.tablet.raw] b0=0x0 b1=0x0 b2=0x0 b3=0x0 b4=0x0 actual=6
```

- `actual=6` proves the interrupt-in transfer completes (6-byte USB tablet report)
- All bytes are zero — the HID report buffer was never populated with host input data
- The condition `b1 != 0x0 || b2 != 0x0 || b3 != 0x0` at sexusb:2734 is never met
- Result: `[sexusb.hid.tablet.nonzero.ok]` never fires

### Stage 3: sexusb forward — PASS (zeroes forwarded correctly)

```
[sexusb.forward.mouse] buttons=0x0 packed=0x0
```

sexusb correctly forwards mouse state. When the tablet reports are zero, it forwards zero. No bug here.

### Stage 4: sexinput — PASS (zeroes processed correctly)

```
[sexinput.mouse.real.delta] dx=0 dy=0 buttons=0x0
```

sexinput receives forwarded data. Zero deltas produce no movement events. No bug here.

### Stage 5: silk-shell — NOT REACHED

No `[shell.cursor.move]` markers observed. The shell never receives non-zero deltas, so cursor position never updates.

### Stage 6: sexdisplay — PASS (draws at initial position)

```
[sexdisplay.cursor.draw] n=0 x=640 y=360
```

Cursor drawn at center (640, 360) — the initial position before any movement. Redraws at same position because no movement events arrive.

---

## 4. Root Cause

**The patched QEMU 9.2.0's XHCI fix does not bridge host input events into the USB HID report buffer.**

- ✅ XHCI controller: Initializes, runs, processes doorbells
- ✅ USB device: Enumerates (slot 1, address assigned)
- ✅ USB HID: Descriptor parsed, interrupt-in endpoint configured
- ✅ Interrupt-in polling: Continuous TRB cycles complete (`actual=6`)
- ❌ HID report data: **Always zero** regardless of host mouse movement

The XHCI fix addresses USB protocol layer issues (register access, TRB handling, doorbell signaling) but does **not** address the QEMU internal pipeline that converts host input events (SDL/GTK) into USB HID report buffers. This pipeline lives in the HID emulation layer (`hw/usb/dev-hid.c`), not the XHCI controller (`hw/usb/hcd-xhci.c`).

This matches the pattern described in `INPUT_SOLVE_PLAN_V1.md`:
> "QEMU 11.0.0 on this host does not deliver host input events to emulated USB HID devices"
> "...but this host/QEMU combo still does not route host events"

The patched QEMU 9.2.0 has the same limitation.

---

## 5. Likely Causes (in order)

| # | Cause | Evidence |
|---|-------|----------|
| 1 | ❌ Patched QEMU XHCI fix doesn't touch HID emulation (`hw/usb/dev-hid.c`) | XHCI OK, but HID reports still zero |
| 2 | ❌ Host input backend (SDL/GTK) not connected to USB HID device model | `QEMU_TRACE` routes event to handler but report buffer not updated |
| 3 | ✅ sexusb enumerates HID but only keyboard boot reports are parsed (N/A — tablet uses report protocol) | Tablet reports decoded correctly (6 bytes) |
| 4 | ✅ sexinput forwards key events but not mouse deltas (unlikely — delta path works for zero data) | Zero deltas processed correctly |
| 5 | ✅ silk-shell receives deltas but cursor update/render path is gated (unlikely — synthetic proof moved cursor) | `[proof.gate.state] enabled=0` — synthetic gate off, real path tested |

---

## 6. Recommended Fix Path

### Track A: Fix patched QEMU's HID report buffer bridge

Modify the patched QEMU's `hw/usb/dev-hid.c` to populate the USB HID report buffer from the `QEMU HID Tablet` event handler. This requires understanding the QEMU internal event routing and USB HID report generation.

**Effort:** High (QEMU source-level fix)
**Scope:** Single file (`hw/usb/dev-hid.c`)
**Risk:** Low for SexOS (all changes in QEMU source)

### Track B: QMP input injection (deterministic, proven)

Use `scripts/qmp_input_probe.py` to inject mouse events via QMP. This bypasses the host-input-to-USB-HID pipeline entirely and provides deterministic input.

From `INPUT_SOLVE_PLAN_V1.md`:
```fish
env SEXOS_QEMU_I8042=off SEXOS_QEMU_QMP=1 SEXUSB_QEMU_DEVICE=kbd ./dev.sh run &
./scripts/qmp_input_probe.py /tmp/sexos-qmp.sock
```

**Effort:** Low (script already exists)
**Limitation:** Not real-time interactive mouse; requires scripted injection

### Track C: virtio-input (alternative device model)

Use `-device virtio-mouse-pci` instead of `-device usb-tablet,bus=xhci.0`. The virtio input path may have different host-input event routing in QEMU that works on this host.

**Effort:** Medium (requires virtio driver in guest)
**Limitation:** Requires sexusb or separate driver to support virtio-input

### Verdict for V1

**Track B (QMP injection)** is the fastest path for deterministic proof work that requires mouse input. **Track A** is the correct permanent fix but requires QEMU source expertise.

---

## 7. No Bell Files Touched

Confirmed: no changes to `servers/sexbell/src/main.rs`, `kernel/src/init.rs`, `crates/sex-pdx/src/lib.rs`, or any Bell handoff documents.

---

## 8. Next Step

Choose fix path (Track A, B, or C) or proceed with `BELL_READER_CAP_FREEZE_V1` while cursor issue is addressed independently.

---

*End of USB_CURSOR_ROUTE_PROOF_V1.md*
