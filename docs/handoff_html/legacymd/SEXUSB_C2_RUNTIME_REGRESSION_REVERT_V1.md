# SEXUSB_C2_RUNTIME_REGRESSION_REVERT_V1

**Date:** 2026-05-07
**Status:** REVERTED — C2A/C2B/C2C removed, C1 restored

## 1. Observed Regression

| Symptom | Detail |
|---------|--------|
| Clock frozen | Counter stops before reaching window-open ticks |
| Windows not opening | Quil, Linen, SilkBar never rendered |
| Runtime gate | FAIL — scheduler not reaching all PDs within probe window |

This is a known pre-existing cooperative-scheduler sensitivity. Any added
work in the hot USB poll loop (C2B match scan, C2C volatile reads) starves
other PDs from receiving scheduler ticks within the gate window.

## 2. Suspected Cause

The C2 patches add code in the **hottest path** of the sexusb poll loop:

| Patch | Added in hot path |
|-------|-------------------|
| C2A | `HidDevice` struct + array (cold — initialization only, not poll loop) |
| C2B | `for midx in 0..device_count` loop on every Transfer Event |
| C2C | 3× `volatile` reads + conditional branches on every slot2 event |

C2A is unlikely to cause regression (initialization only). C2B and C2C
add per-iteration work in the innermost event-loop spin. Even budgeted
to 32/16 markers, the `for` loop and volatile reads consume cycles that
the cooperative scheduler doesn't get back.

Real fix requires either:
- A preemptive scheduler (kernel change, STOP FIRST)
- Moving slot2 handling to a separate PD (architectural, STOP FIRST)
- Using the sexusb synthetic gate to inject slot2 data without poll-loop changes

## 3. Commits Reverted

```
4915528 Revert "usb: classify slot2 tablet reports without dispatch" (C2C)
1e46418 Revert "usb: identify slot2 transfer events without dispatch"    (C2B)
91a0c8f Revert "usb: add bounded HID device table for slot2 demux"       (C2A)
```

## 4. C1 Baseline Restored

| Check | Status |
|-------|--------|
| `[sexusb.slot2.poll.start]` marker present | ✅ 1 occurrence |
| `HidDevice` / C2 markers absent | ✅ Only pre-existing comment |
| Build passes | ✅ 1761 sectors |
| `single_bind` based poll loop intact | ✅ |
| No sexinput/shell/display/kernel edits | ✅ |

## 5. Build Result

```
./scripts/entrypoint_build.sh → PASS, 1761 sectors, 0 errors
```

## 6. Next Steps for USB Agent

**Do not continue C2D (rearm) or C2E (forward).**

The USB multi-device demux path is blocked by cooperative-scheduler
sensitivity. Options:

1. **Preemptive scheduler** — kernel change, out of scope for USB agent.
2. **Separate sexusb2 PD** — second USB PD for slot2. Architectural,
   requires kernel devmgr + PDX slot changes.
3. **Synthetic gate for slot2** — Use the existing `SEXUSB_SYNTHETIC`
   gate to inject slot2 pointer data via `OP_USB_MOUSE_REPORT` without
   touching the poll loop. This bypasses the hot-path regression.
4. **Real hardware boot** — May not exhibit the same scheduler starvation
   since timing differs from QEMU.

**C1 is the safe USB stopping point.** The single TRB queued for slot2
proves the ring + endpoint work. The poll loop regression is a system
problem, not a USB driver bug.

## 7. Files Changed

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | Reverted to C1-only state |
| `docs/handoff/SEXUSB_HID_DEVICE_TABLE_C2A_V1.md` | Deleted (revert) |
| `docs/handoff/SEXUSB_EVENT_MATCH_HELPER_C2B_V1.md` | Deleted (revert) |
| `docs/handoff/SEXUSB_SLOT2_CLASSIFY_NO_DISPATCH_C2C_V1.md` | Deleted (revert) |
| `docs/handoff/SEXUSB_C2_RUNTIME_REGRESSION_REVERT_V1.md` | Created |

---

*End of SEXUSB_C2_RUNTIME_REGRESSION_REVERT_V1.md*
