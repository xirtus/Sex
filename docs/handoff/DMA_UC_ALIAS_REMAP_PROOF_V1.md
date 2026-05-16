# DMA_UC_ALIAS_REMAP_PROOF_V1

**Status:** PASS IMPLEMENTED — 138/138 gates, 0 faults.
**Date:** 2026-05-17

---

## Result: Both rings UC-remapped via alias. rx_alias=1, tx_alias=1.

---

## Alias Mapping Table

| Ring | Physical | HHDM (WB) | UC Alias | Flags | OK |
|------|----------|-----------|----------|-------|----|
| RX | 0x1F880000 | 0xFFFF80001F880000 | 0xFFFF90001F880000 | NO_CACHE\|WRITE_THROUGH | ✅ |
| TX | 0x102AA000 | 0xFFFF8000102AA000 | 0xFFFF9000102AA000 | NO_CACHE\|WRITE_THROUGH | ✅ |

TLB flushed for both aliases. HHDM mapping preserved unchanged. mmio_writes=0, dma=0, rings_enabled=0, packets=0.

---

## Files: kernel +40 (pci.rs: imports, map_physical_range calls, TLB flush, markers)

## Proof: 138/138 PASS, 0 faults (was 137)

## Fault Count: **0**

## Next: MMIO ring base write plan → RX/TX enable STOP review

## Commit
```bash
git add kernel/src/hal/pci.rs docs/handoff/DMA_UC_ALIAS_REMAP_PROOF_V1.md
git commit -m "feat(net): DMA UC alias remap proof V1"
```
