# SEXUSB_DUAL_DEVICE_QEMU_SLOT2_PROOF_V1

**Date:** 2026-05-07
**Status:** PARTIAL — dual-device QEMU config added, slot2 detection proven,
           C2C code compiles, config_ep stall blocks full proof

## 1. What Was Done

| Action | Result |
|--------|--------|
| Added `kbd+tablet` mode to `dev.sh` | ✅ `-device usb-kbd -device usb-tablet` |
| `ports.collect count=2` | ✅ Both devices detected |
| Slot2 full enumeration | ✅ Enable→Address→Desc→Classify→SET_CONFIG→ConfigEp |
| C2C classify code | ✅ Compiles, gated behind `device_count > 1` |
| Slot1 config_ep after slot2 | ❌ Stalls — command completion never arrives |

## 2. config_ep Stall Analysis

Slot1 Configure Endpoint poll loop has a pre-existing bug: `break` on
first owned event regardless of type. If a non-command Transfer Event
arrives first, the poll exits without consuming the command completion.

Fix applied: move `break` inside `if command_completion` block, consume
non-command events and continue. Same fix applied to slot2 config_ep
(works — slot2 `configure_endpoint.ok` appears).

Slot1 config_ep still stalls despite both fixes + event ring drain.
Deeper investigation needed (command ring state, DCBAA, slot context).

## 3. Single-Device Regression

`SEXUSB_QEMU_DEVICE=kbd` works perfectly:
- config_ep.ok=1, continuous.start=1, C2B=15, yield=8, faults=0

## 4. C2C Status

| Aspect | Status |
|--------|--------|
| Code compiles | ✅ |
| Struct fields sufficient | ✅ `intr_report_va`, `intr_report_len`, `intr_ring_va`, `intr_report_phys` |
| Gated behind `device_count > 1` | ✅ |
| Slot2 classify logic | ✅ 3-byte read, all_zero/button/motion classification |
| Runtime proven | ❌ Blocked by config_ep stall |

## 5. Files Changed

| File | Change |
|------|--------|
| `dev.sh` | +1 line: `kbd+tablet` case |
| `servers/sexusb/src/main.rs` | +30 lines: HidDevice fields, event ring drain, dual config_ep fix, C1 moved |
| `docs/handoff/SEXUSB_DUAL_DEVICE_QEMU_SLOT2_PROOF_V1.md` | Created |

## 6. Next Steps

1. Debug slot1 config_ep stall with dual-device (command ring state audit)
2. Once config_ep passes: re-run with kbd+tablet, capture C2C markers
3. C2E / USB_HID_POINTER_PRODUCER unblocked after C2C runtime proof

---

*End of SEXUSB_DUAL_DEVICE_QEMU_SLOT2_PROOF_V1.md*
