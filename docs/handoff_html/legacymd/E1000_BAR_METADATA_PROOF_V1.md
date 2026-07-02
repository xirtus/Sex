# E1000_BAR_METADATA_PROOF_V1

**Status:** PASS IMPLEMENTED — 130/130 gates, 0 faults.
**Date:** 2026-05-16

---

## BAR0 Metadata

| Field | Value |
|-------|-------|
| Vendor | 0x8086 (Intel) |
| Device | 0x10D3 (82574L) |
| BAR0 type | Memory (MMIO) |
| BAR0 base | 0xFEB80000 |
| BAR0 size | 32-bit |
| Mapped | 0 |
| Size probe | 0 (needs BAR write) |
| Driver attached | 0 |
| MMIO access | 0 |
| IRQ | 0 |
| DMA | 0 |
| Packets | 0 |

## Files: kernel +8 (pci.rs), silk-shell +9, master_gate +10, run_proof +1

## Proof: 130/130 PASS, 0 faults (was 129)

## Next: E1000_MMIO_MAP_PLAN_V1

## Commit
```bash
git add kernel/src/hal/pci.rs servers/silk-shell/src/main.rs scripts/daily_driver_master_gate.sh scripts/run_daily_driver_proof.sh docs/handoff/E1000_BAR_METADATA_PROOF_V1.md
git commit -m "feat(pci): e1000 BAR metadata proof V1"
```
