# E1000_MMIO_MAP_PROOF_V1

**Status:** PASS IMPLEMENTED — 130/130 gates, 0 faults.
**Date:** 2026-05-16

---

## Result: MMIO read-only probe successful. BAR0 at 0xFEB80000 → virt 0xFFFF8000FEB80000. Register 0x0 = 0x00140241.

---

## MMIO Probe Table

| Field | Value |
|-------|-------|
| BAR0 physical | 0xFEB80000 |
| BAR0 virtual | 0xFFFF8000FEB80000 |
| Offset | 0x0000 (device control) |
| Raw value | 0x00140241 |
| Read | 1 |
| Write | 0 |
| Mapped | 1 (read-only) |
| Driver | 0 |
| IRQ | 0 |
| DMA | 0 |
| Packets | 0 |

## Files: kernel +7 (pci.rs)

## Proof: 130/130 PASS, 0 faults

## Fault Count: **0**

## Next: E1000_MAC_READ_PROOF_V1

## Commit
```bash
git add kernel/src/hal/pci.rs docs/handoff/E1000_MMIO_MAP_PROOF_V1.md
git commit -m "feat(pci): e1000 MMIO read-only probe V1"
```
