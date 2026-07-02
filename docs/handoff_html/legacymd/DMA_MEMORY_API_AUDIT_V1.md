# DMA_MEMORY_API_AUDIT_V1

**Status:** PASS REVIEW ONLY — Slab allocator exists, can provide DMA-suitable memory.
**Date:** 2026-05-16

---

## Key Finding: Slab allocator backed by 4K page frames exists

`kernel/src/slab.rs` — slab allocator uses `FrameAllocator<Size4KiB>`. Provides 64/128/512-byte slabs. Identity-mapped (phys = virt - HIGH_HALF_BASE). Suitable for static e1000 descriptor rings.

---

## Readiness Table

| Capability | Status | Notes |
|-----------|--------|-------|
| Frame allocator | ✅ | 4K page frames via `allocate_frame()` |
| Contiguous allocation | ✅ | Single 4K page per slab refill |
| Physical address | ✅ | virt - HIGH_HALF_BASE gives phys |
| 16-byte alignment | ✅ | Slab provides aligned blocks |
| Identity mapping | ✅ | Higher-half: phys + 0xFFFF800000000000 |
| Pin/lifetime | ✅ | Static allocation at boot, no dealloc needed |
| Cache attributes | ⚠️ | Default WB; UC/WC not configured for DMA |
| Zeroing | ❓ | Not confirmed in slab |
| MPK/PKU ownership | ❌ | No memory protection domain separation |
| Deallocation | ❌ | Not implemented; static-only is fine for rings |
| Precedent | ✅ | LAPIC/IOAPIC/framebuffer use identity mapping |

---

## Recommended: **B — DMA_STATIC_RING_BUFFER_PLAN_V1**

Static allocation of descriptor rings at boot using kernel slab allocator. Single 4K page can hold 256 e1000 descriptors (16 bytes each). No dynamic alloc/dealloc. Physical address via virt-to-phys conversion.

---

## STOP FIRST Boundaries (all pass for static plan)

| Boundary | Status |
|----------|--------|
| Memory manager changes | ❌ Not needed |
| Unbounded allocation | ❌ Static, bounded |
| DMA without phys certainty | ❌ Identity-mapped, phys known |
| Cache coherency | ⚠️ Monitor — may need UC for DMA regions |
| MMIO writes | ❌ Deferred |
| Packets | ❌ Deferred |

---

## Next: DMA_STATIC_RING_BUFFER_PLAN_V1

## Commit
```bash
git add docs/handoff/DMA_MEMORY_API_AUDIT_V1.md
git commit -m "docs(kernel): DMA memory API audit V1"
```
