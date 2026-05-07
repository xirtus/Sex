# USB_100_PLAN_V1

**Date:** 2026-05-07
**Status:** PLAN — Option E (synthetic gate) first, Option B (budgeted yield) second

## Root Cause

USB is not primarily XHCI complexity. It is **input proof under scheduler
budget**. C1 proved the scheduler survives when sexusb does almost no
hot-path work. C2B/C2C proved extra poll-loop work starves cooperative
scheduling.

## Finish Line (USB 100%)

```
1.  C1 baseline still boots with no freeze.
2.  Synthetic slot2 report moves cursor through existing HID path.
3.  Synthetic button edge focuses/clicks shell surface.
4.  Budgeted poll loop survives 30s+ with clock/windows alive.
5.  Real QEMU USB tablet/mouse interrupt-IN report reaches normalizer.
6.  Real report becomes OP_HID_EVENT.
7.  silk-shell receives movement + button edge.
8.  Cursor moves visibly.
9.  Click focuses a window.
10. Drag moves/resizes according to existing shell policy.
11. No #PF/#GP/panic.
12. No IPC storm.
13. No broad kernel/scheduler/ABI refactor.
14. Handoff documents recurring scheduler sensitivity.
```

## Phase Order

### Phase E — Synthetic Slot2 Gate (NOW)

Prove pointer/click/focus path **without touching the poll loop**.

Route: `synthetic report → HID_POINTER_REPORT_NORMALIZER_V1 → OP_HID_EVENT → silk-shell pointer state → click focus / drag`

### Phase B — Budgeted Poll Yield (SECOND)

Make sexusb poll-loop work scheduler-safe with strict per-iteration
budget/yield discipline. No preemptive scheduler — just bounded work
per loop iteration.

### Phase C2-Restart — Real USB Demux (THIRD)

After E+B pass:
1. C2B_RESTART_BUDGETED_ENDPOINT_EVENT_V1
2. C2C_RESTART_BUDGETED_REPORT_FETCH_V1
3. USB_HID_BOOT_MOUSE_REPORT_V1
4. USB_HID_POINTER_PRODUCER_V1
5. Real hardware/QEMU tablet proof
6. Later: touchpad absolute contacts
7. Much later: gestures

## Architecture Boundaries

- `sexusb` = USB bus owner only (not input policy, not compositor)
- `sexinput` = input meaning (normalizer, event routing)
- `silk-shell` = focus/click/drag policy
- `sexdisplay` = sole framebuffer writer
- Keep sexusb narrow — do not become a broad sexlink monolith

## Known-Good Pipeline

```
raw/synthetic report
  → HID_POINTER_REPORT_NORMALIZER_V1
  → OP_HID_EVENT
  → silk-shell pointer state
  → click focus / drag
```

---

*End of USB_100_PLAN_V1.md*
