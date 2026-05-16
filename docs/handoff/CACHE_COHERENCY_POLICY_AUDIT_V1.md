# CACHE_COHERENCY_POLICY_AUDIT_V1

**Status:** PASS REVIEW ONLY — UC mapping already supported.
**Date:** 2026-05-16

---

## Key Finding: UC (Uncacheable) page-table flags already exist

`kernel/src/syscalls/mod.rs:364`: `PageTableFlags::NO_CACHE | PageTableFlags::WRITE_THROUGH` — PCD=1 + PWT=1 selects UC in default PAT. Already used for MMIO BAR mapping via `map_physical_range()`.

---

## DMA Policy Table

| Region | Recommended Cache | Status |
|--------|-----------------|--------|
| Descriptor rings (RX/TX) | UC | ✅ `NO_CACHE \| WRITE_THROUGH` supported |
| RX packet buffers | UC or WB+barrier | UC safest |
| TX packet buffers | UC or WB+barrier | UC safest |
| MMIO BAR (0xFEB80000) | UC | ✅ Already working (higher-half, implicit) |

---

## QEMU Tolerance

QEMU e1000 is tolerant of WB (write-back) guest memory. UC mapping works and is safer. WC (write-combining) is optional optimization for TX.

---

## Recommended: **A — DMA_STATIC_RING_ALLOCATION_PROOF_V1**

Allocate and map descriptor rings with UC flags via `map_physical_range()`. Marker-only — no MMIO writes, no RX/TX enable, no packets. Prove physical addresses and UC mapping.

---

## STOP FIRST Boundaries

| Boundary | Status |
|----------|--------|
| Enabling DMA without coherency | ❌ UC mapping resolves this |
| MMIO writes | ❌ Deferred |
| RX/TX enable | ❌ Deferred |
| Packet send | ❌ Deferred |
| Page-table refactor | ❌ Not needed |

---

## Next: DMA_STATIC_RING_ALLOCATION_PROOF_V1

## Commit
```bash
git add docs/handoff/CACHE_COHERENCY_POLICY_AUDIT_V1.md
git commit -m "docs(kernel): cache coherency policy audit V1"
```
