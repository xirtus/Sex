# E1000_BAR_METADATA_AUDIT_V1

**Status:** PASS REVIEW ONLY — `get_bar()` already exists, can read e1000 BAR.
**Date:** 2026-05-16

---

## Key Finding: BAR reading is fully implemented

`kernel/src/hal/pci.rs::get_bar(index)` at line 25 reads BAR at offset 0x10 + index*4, handles 64-bit BARs, returns base address. Already used for GPU and NVMe. Can be called for e1000 without any new infrastructure.

---

## BAR Audit Table

| Capability | Status |
|-----------|--------|
| BAR config read | ✅ `get_bar()` |
| Memory vs I/O decode | ✅ `bar & 0x1` |
| 32-bit vs 64-bit decode | ✅ `(bar >> 1) & 0x3` |
| Prefetchable bit | ✅ Bit 3 in BAR |
| Size probing (write 0xFFFFFFFF, read back) | ❌ Not implemented |
| MMIO mapping | ❌ Not implemented |
| Register read/write via BAR | ❌ Not implemented |

---

## Recommended: **A — E1000_BAR_METADATA_PROOF_V1**

Add marker in `enumerate_bus()` that reads BAR0 for class 0x02 devices. Marker-only — no MMIO map, no register access. Reports BAR0 value, type (memory/IO), and size category (32/64-bit).

---

## STOP FIRST Boundaries

| Boundary | Status |
|----------|--------|
| BAR register writes | ❌ Blocked |
| MMIO mapping | ❌ Blocked |
| MMIO register R/W | ❌ Blocked |
| IRQ/DMA/rings | ❌ Blocked |
| Driver attach | ❌ Blocked |
| Packet send | ❌ Blocked |

---

## Next: E1000_BAR_METADATA_PROOF_V1

## Commit
```bash
git add docs/handoff/E1000_BAR_METADATA_AUDIT_V1.md
git commit -m "docs(net): e1000 BAR metadata audit V1"
```
