# DMA_STATIC_RING_BUFFER_PLAN_V1

**Status:** PASS REVIEW ONLY — Docs-only plan.
**Date:** 2026-05-16
**Gates:** 131/131 baseline.

---

## Static Ring Strategy

| Resource | Size | Count |
|----------|------|-------|
| RX descriptor page | 4K (1 frame) | 256 descriptors × 16 bytes |
| TX descriptor page | 4K (1 frame) | 256 descriptors × 16 bytes |
| RX packet buffers | 2K each | 8–16 buffers (static array) |
| TX packet buffers | 2K each | 4–8 buffers (static array) |

All allocated at boot via slab allocator. Identity-mapped. No dynamic free. Kernel/sexnet ownership only.

---

## Address Model

- Virtual: `HIGH_HALF_BASE + phys` (identity map)
- Physical: `virt - HIGH_HALF_BASE`
- Ring base registers (e1000 RDBA/TDBA) take physical addresses
- Direct kernel/sexnet access via virtual addresses

---

## Cache Coherency Options

| Option | Risk | Status |
|--------|------|--------|
| UC (Uncacheable) | Safe but slow | Needs MTRR/PAT config |
| WC (Write-Combining) | Good for TX | Needs MTRR/PAT config |
| WB + barriers | Risky without explicit flush | Not recommended initially |
| Status quo (default WB) | May work on QEMU | Monitor |

---

## Phase Ladder

| Phase | What |
|-------|------|
| 0 | This plan |
| 1 | Descriptor format spec |
| 2 | Static ring allocation stub (no MMIO writes) |
| 3 | Physical address proof |
| 4 | Cache policy decision |
| 5 | MMIO ring base write plan |
| 6 | RX enable plan |
| 7 | **TX STOP review** before packet |
| 8 | ARP/IP later |

---

## Next: E1000_DESCRIPTOR_FORMAT_SPEC_V1

## Commit
```bash
git add docs/handoff/DMA_STATIC_RING_BUFFER_PLAN_V1.md
git commit -m "docs(net): DMA static ring buffer plan V1"
```
