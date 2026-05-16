# UC_PTE_REMAP_PLAN_V1

**Status:** PASS REVIEW ONLY — Docs-only plan.
**Date:** 2026-05-17
**Gates:** 137/137 baseline.

---

## Plan: Remap RX/TX ring pages with UC (NO_CACHE | WRITE_THROUGH)

Rings allocated (RX phys=0x1F880000, TX phys=0x102AA000) — currently mapped WB via HHDM. Need UC remap before DMA enable. No MMIO writes until remap proven.

---

## Required API/Flags

| Flag | Purpose |
|------|---------|
| PRESENT | Page present |
| WRITABLE | Kernel write access |
| NO_CACHE (PCD) | Disable cache |
| WRITE_THROUGH (PWT) | Write-through (PCD+PWT = UC in default PAT) |
| NX (NO_EXECUTE) | Optional, recommended |

Existing `map_physical_range()` in `kernel/src/syscalls/mod.rs` already supports these flags.

---

## Alias vs In-Place Decision

| Option | Risk |
|--------|------|
| **Alias mapping** (map same phys at new virt with UC) | Safe — preserves HHDM WB mapping |
| In-place HHDM remap (change existing HHDM PTE flags) | ⚠️ Changes global HHDM, affects all PDs |
| **Recommended: Alias** — map at `HHDM_BASE + alias_offset + phys` with UC flags | |

---

## Phase Ladder

| Phase | What |
|-------|------|
| 0 | This plan |
| 1 | DMA UC remap API audit |
| 2 | Alias-vs-in-place decision |
| 3 | UC remap proof marker (no DMA) |
| 4 | Packet buffer allocation plan |
| 5 | Packet buffer UC mapping proof |
| 6 | MMIO ring base write plan |
| 7 | **RX/TX enable STOP review** |
| 8 | **Packet STOP review** |

---

## STOP FIRST Boundaries

In-place HHDM remap without audit, WB+UC aliasing unresolved, no TLB flush, MMIO writes, RX/TX enable, packets, Browser grant, page table refactor

---

## Next: DMA_UC_REMAP_API_AUDIT_V1

## Commit
```bash
git add docs/handoff/UC_PTE_REMAP_PLAN_V1.md
git commit -m "docs(net): UC PTE remap plan V1"
```
