# DMA_UC_REMAP_API_AUDIT_V1

**Status:** PASS REVIEW ONLY — All APIs exist for UC alias mapping.
**Date:** 2026-05-17

---

## Key Finding: `map_physical_range` + `invlpg` + UC flags all exist

- `GLOBAL_VAS.map_physical_range(VirtAddr, phys, size, flags, pku_key)` ✅
- `PageTableFlags::NO_CACHE | PageTableFlags::WRITE_THROUGH` ✅
- `invlpg` TLB flush ✅ (kernel/src/pku.rs)

---

## API Readiness Table

| Capability | Status |
|-----------|--------|
| map_physical_range | ✅ syscalls/mod.rs:278,367,457 |
| UC flags (NO_CACHE \| WRITE_THROUGH) | ✅ syscalls/mod.rs:364-366 |
| Alias virt address | ⚠️ Need dedicated range (e.g., 0xFFFF_9000_0000_0000) |
| TLB flush (invlpg) | ✅ pku.rs |
| PRESENT \| WRITABLE \| NX | ✅ All available |
| HHDM WB preserved | ✅ Alias mapping leaves HHDM intact |
| No page table refactor | ✅ Existing API sufficient |

---

## Recommended Alias Range

```
HHDM (WB):      0xFFFF_8000_0000_0000 + phys  (existing)
DMA UC alias:   0xFFFF_9000_0000_0000 + phys  (proposed)
```

RX ring: phys=0x1F880000 → UC alias virt=0xFFFF_9000_1F88_0000
TX ring: phys=0x102AA000 → UC alias virt=0xFFFF_9000_102A_A000

---

## Recommended: **A — DMA_UC_ALIAS_REMAP_PROOF_V1**

Map both rings at UC alias virtual addresses using `map_physical_range()` with `NO_CACHE | WRITE_THROUGH`. Flush TLB. Emit proof markers. No MMIO writes. No DMA enable.

---

## STOP FIRST Boundaries (all pass)

| Boundary | Status |
|----------|--------|
| Unknown alias range | ❌ 0xFFFF_9000 is available |
| No TLB flush | ❌ invlpg exists |
| Flags unavailable | ❌ All flags present |
| Page table refactor | ❌ Not needed |
| MMIO writes / DMA / packets | ❌ All deferred |

---

## Next: DMA_UC_ALIAS_REMAP_PROOF_V1

## Commit
```bash
git add docs/handoff/DMA_UC_REMAP_API_AUDIT_V1.md
git commit -m "docs(net): DMA UC remap API audit V1"
```
