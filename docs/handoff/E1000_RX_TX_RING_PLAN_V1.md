# E1000_RX_TX_RING_PLAN_V1

**Status:** PASS REVIEW ONLY — Docs-only plan.
**Date:** 2026-05-16
**Gates:** 131/131 baseline.

---

## Plan: Bounded RX/TX descriptor rings + DMA buffers for e1000

Kernel/sexnet ownership only. No Browser direct NIC. No packets until STOP review.

---

## DMA/Memory Questions

| Question | Status |
|----------|--------|
| Physically contiguous memory | Needed for descriptor rings |
| 16-byte alignment | e1000 requirement for descriptors |
| DMA-safe physical addresses | Identity mapped (higher-half) |
| Cache coherency | UC/WC needed for DMA regions |
| Page pinning | Not needed (no paging to disk) |
| MPK/PKU ownership | Kernel-only initially, sexnet later |
| Ring lifetime | Static allocation, no dynamic resize |

---

## Phase Ladder

| Phase | What | Writes? |
|-------|------|---------|
| 0 | This plan | No |
| 1 | DMA memory API audit | No |
| 2 | Descriptor format spec | No |
| 3 | Ring allocation stub (no MMIO writes) | No |
| 4 | MMIO write plan for ring base registers | Plan only |
| 5 | RX enable plan | Plan only |
| 6 | **TX test-frame STOP review** | Review only |
| 7 | ARP/IP plan | Future |
| 8 | TCP/HTTP later | Future |

---

## Future Markers

`[e1000.ring.plan]` `[e1000.dma.memory.audit]` `[e1000.rx.ring.stub]` `[e1000.tx.ring.stub]` `[e1000.ring.truth]` `[e1000.packet.stop_review]`

---

## STOP FIRST Boundaries

DMA buffer allocation, MMIO write, interrupt enable, RX/TX enable, packet send, Browser network grant, unbounded buffers, cache coherency uncertainty, broad memory manager changes

---

## Next: DMA_MEMORY_API_AUDIT_V1

## Commit
```bash
git add docs/handoff/E1000_RX_TX_RING_PLAN_V1.md
git commit -m "docs(net): e1000 RX/TX ring plan V1"
```
