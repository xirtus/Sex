# SEXUSB_XHCI_CONFIG_EP_CMD_RING_AUDIT_V1

**Date:** 2026-05-07
**Status:** FIXED — config_ep stall resolved, dual-device boots

## Root Cause

Slot2 enumeration (Enable Slot → Address → Descriptor → SET_CONFIG →
Configure Endpoint) consumed command ring and event ring entries before
slot1 Configure Endpoint ran.  The config_ep poll loop had a `break`
outside the `if command_completion` block, causing it to exit on any
owned event (not just command completions).  Residual events from
slot2 enumeration blocked slot1 config_ep.

## Fix

Two changes:
1. **Reorder**: slot1 Configure Endpoint now runs BEFORE slot2 enumeration.
   Clean command/event ring state when slot1 config_ep polls.
2. **Poll fix**: `break` moved inside `if command_completion` block in
   both config_ep poll loops.  Non-command events consumed and loop
   continues.

## Runtime Proof

| Test | config_ep.ok | continuous | C2B | C2C | Yield | Faults |
|------|-------------|------------|-----|-----|-------|--------|
| Single (kbd) | ✅ 1 | ✅ 1 | 15 | 0 | 8 | 0 |
| Dual (kbd+tablet) | ✅ 1 | ✅ 1 | 15 | 0 | 16 | 0 |

## C2C Status

C2C=0 because QEMU usb-tablet produces zero interrupt-IN completions
in headless mode (`-display none`).  C2C code is correctly gated and
will fire when tablet data arrives (via interactive QEMU, QMP, or
real hardware).  Code-complete, not runtime-proven due to QEMU data gap.

## Files Changed

| File | Change |
|------|--------|
| `servers/sexusb/src/main.rs` | ~150-line reorder + config_ep poll fixes |
| `dev.sh` | +1: `kbd+tablet` mode |
| `docs/handoff/SEXUSB_XHCI_CONFIG_EP_CMD_RING_AUDIT_V1.md` | Created |

## USB 100% Progress

| # | Item | Status |
|---|------|--------|
| 1 | C1 baseline | ✅ |
| 2 | Synthetic slot2 | ✅ |
| 3 | Shell click/focus | ⚠️ Partial |
| 4 | Budgeted yield | ✅ |
| 5 | C2B event match | ✅ |
| 6 | Dual-device detection | ✅ |
| 7 | Dual-device config_ep | ✅ FIXED |
| 8 | C2C classify (code) | ✅ Compiles |
| 9 | C2C classify (runtime) | ⬜ Needs tablet data source |
| 10 | C2E pointer forward | ⬜ After C2C runtime proof |

---

*End of SEXUSB_XHCI_CONFIG_EP_CMD_RING_AUDIT_V1.md*
