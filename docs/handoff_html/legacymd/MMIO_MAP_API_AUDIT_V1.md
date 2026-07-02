# MMIO_MAP_API_AUDIT_V1

**Status:** PASS REVIEW ONLY — Higher-half identity mapping already exists.
**Date:** 2026-05-16

---

## Key Finding: No new API needed. Physical memory is identity-mapped.

`HIGH_HALF_BASE = 0xFFFF_8000_0000_0000` — all physical addresses accessible at `HIGH_HALF_BASE + phys`.

---

## Readiness Table

| Capability | Status | Evidence |
|-----------|--------|----------|
| Phys→virt mapping | ✅ | `HIGH_HALF_BASE + phys` identity map |
| MMIO precedent | ✅ | LAPIC, IOAPIC, framebuffer |
| Framebuffer access | ✅ | `FB_PTR` at higher-half, `write_volatile` |
| read_volatile/write_volatile | ✅ | Used in framebuffer pixel loop |
| Cache-disable (UC) | ⚠️ | Not explicitly set; framebuffer uses WC, LAPIC uses UC |
| Read-only mapping | ❌ | Not enforced — voluntary |
| Kernel-only access | ❌ | No PKU separation for MMIO |
| Unmap/size probe write | ❌ | Not implemented |

---

## e1000 BAR0 Virtual Address

```
BAR0 phys = 0xFEB80000 → virt = 0xFFFF_8000_FEB8_0000
```

Accessible via `core::ptr::read_volatile(virt_addr)` — same pattern as framebuffer.

---

## Precedents

| Device | Mapping | Access Pattern |
|--------|---------|---------------|
| LAPIC | `physical_memory_offset + lapic_addr` | `read_volatile` / `write_volatile` |
| IOAPIC | `physical_memory_offset + io_apic_addr` | `read_volatile` / `write_volatile` |
| Framebuffer | `FB_PTR` at higher-half | `write_volatile` pixel loop |

---

## Recommended: **B — E1000_MMIO_MAP_PROOF_V1**

Add kernel marker that reads e1000 device ID register (offset 0x0000) via `read_volatile` at `HIGH_HALF_BASE + BAR0`. Read-only probe. No writes. No IRQ. No DMA.

---

## STOP FIRST Boundaries (all pass for read-only)

| Boundary | Status |
|----------|--------|
| Page table changes | ❌ Not needed |
| Cache attribute uncertainty | ⚠️ Monitor — framebuffer uses WC, LAPIC uses UC |
| MMIO writes | ❌ Read-only only |
| IRQ/DMA/packets | ❌ None |
| Browser access | ❌ Kernel-only |

---

## Next: E1000_MMIO_MAP_PROOF_V1

## Commit
```bash
git add docs/handoff/MMIO_MAP_API_AUDIT_V1.md
git commit -m "docs(kernel): MMIO map API audit V1"
```
