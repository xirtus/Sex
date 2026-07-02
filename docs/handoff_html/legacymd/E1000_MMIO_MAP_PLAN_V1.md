# E1000_MMIO_MAP_PLAN_V1

**Status:** PASS REVIEW ONLY — Docs-only plan.
**Date:** 2026-05-16
**Gates:** 130/130 baseline.

---

## Plan: Map e1000 BAR0 MMIO (0xFEB80000) read-only for kernel probe

No writes, no IRQ, no DMA, no packets. Browser never touches NIC.

---

## Required OS Facilities

| Facility | Need |
|----------|------|
| Physical MMIO mapping API | Map 0xFEB80000 into kernel address space |
| Page table flags | Uncachable (UC) or write-combining (WC) |
| PKU/MPK | Kernel-only access; sexnet gets mapped view later |
| BAR0 size | ~128KB typical for e1000; needs size probe first |

---

## Phase Ladder

| Phase | What |
|-------|------|
| 0 | This plan |
| 1 | MMIO map API audit |
| 2 | Read-only register probe plan |
| 3 | Map BAR0 for kernel probe only |
| 4 | Read device ID/STATUS registers |
| 5 | MAC address read (RAL/RAH) |
| 6 | sexnet driver ownership plan |
| 7 | DMA/ring plan |
| 8 | **STOP REVIEW** before any write/packet |

---

## Future Markers

`[e1000.mmio.map.plan]` `[e1000.mmio.map.audit]` `[e1000.mmio.probe]` `[e1000.register.read]` `[e1000.mac.read]` `[e1000.mmio.truth]`

---

## STOP FIRST Boundaries

MMIO write, BAR size probe write, IRQ, DMA/rings, packet send, Browser direct NIC, kernel/ABI edits

---

## Next: MMIO_MAP_API_AUDIT_V1

## Commit
```bash
git add docs/handoff/E1000_MMIO_MAP_PLAN_V1.md
git commit -m "docs(net): e1000 MMIO map plan V1"
```
