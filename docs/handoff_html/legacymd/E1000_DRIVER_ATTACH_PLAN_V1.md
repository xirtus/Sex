# E1000_DRIVER_ATTACH_PLAN_V1

**Status:** PASS REVIEW ONLY — Docs-only plan.
**Date:** 2026-05-16
**Gates:** 129/129 baseline.

---

## Attach Plan: Intel 82574L (e1000-family) driver

Read BAR metadata, probe MMIO, read MAC — all without packets. No RX/TX until STOP review.

---

## Phase Ladder

| Phase | What | Packets? |
|-------|------|----------|
| 0 | This plan | No |
| 1 | BAR metadata audit (read BAR0/BAR1 sizes, types from PCI config) | No |
| 2 | MMIO map plan (reserve address space, no writes) | No |
| 3 | Register read-only probe (STATUS, EERD, CTRL — verify device alive) | No |
| 4 | MAC address read (RAL/RAH or EEPROM read) | No |
| 5 | Driver status: attached=1, RX/TX=0 | No |
| 6 | RX/TX ring allocation plan | No |
| 7 | **STOP REVIEW** before any packet send | No |
| 8 | ARP/IP/TCP later | Future |

---

## Safety Invariants

- Browser never gets direct NIC access
- sexnet owns future NIC driver
- Collar grants browser network capability later
- No ambient network authority
- No packets before explicit STOP review
- Bounded MMIO/register access only
- No IRQ handling until separate review
- No DMA until separate review
- No kernel/sex-pdx/global ABI edits
- No heap/std/libc/thread dependency

---

## Future Markers

`[e1000.attach.plan]` `[e1000.bar.metadata]` `[e1000.mmio.probe]` `[e1000.mac.read]` `[e1000.driver.truth]` `[e1000.attach.done]`

---

## STOP FIRST Boundaries

BAR mapping, MMIO writes, IRQ, DMA/ring, packet send, Browser SLOT_NET grant, kernel/ABI edits

---

## Next: E1000_BAR_METADATA_AUDIT_V1

## Commit
```bash
git add docs/handoff/E1000_DRIVER_ATTACH_PLAN_V1.md
git commit -m "docs(net): e1000 driver attach plan V1"
```
