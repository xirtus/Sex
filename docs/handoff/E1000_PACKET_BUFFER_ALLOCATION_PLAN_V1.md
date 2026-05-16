# E1000_PACKET_BUFFER_ALLOCATION_PLAN_V1

**Status:** PASS REVIEW ONLY — Docs-only plan.
**Date:** 2026-05-17
**Gates:** 138/138 baseline.

---

## Plan: 16 packet buffers (8 RX + 8 TX), 2K each, 4K pages, UC aliased.

---

## Buffer Strategy

| Parameter | Value |
|-----------|-------|
| RX buffers | 8 |
| TX buffers | 8 |
| Buffer size | 2048 bytes |
| Page size | 4096 (2 buffers per page) |
| Total pages | 8 (16 buffers) |
| Alignment | 4K (page-aligned) |
| UC alias | Same pattern as rings: 0xFFFF9000 + phys |
| Zero init | Via HHDM before DMA enable |

## Ownership

| Phase | RX Owner | TX Owner |
|-------|---------|---------|
| Allocation | Driver | Driver |
| After RX enable | Device (DMA writes) | Driver |
| Before TX send | Driver (fills buffer) | Driver |
| After TX send | Device (DMA reads) | Device |
| Browser | **Never** | **Never** |

---

## Metadata Per Buffer

phys, hhdm_virt, uc_alias, len=2048, owner=driver, in_use=0, device_visible=0

---

## Cache/TLB Policy

NO_CACHE | WRITE_THROUGH flags, TLB flush per buffer alias, zero via HHDM before UC alias setup, avoid WB writes after UC mapping

---

## Phase Ladder

| Phase | What |
|-------|------|
| 0 | This plan |
| 1 | Packet buffer allocation proof |
| 2 | UC alias proof |
| 3 | Descriptor links to buffer phys (no MMIO writes) |
| 4 | MMIO ring base write plan |
| 5 | RX enable plan |
| 6 | **TX packet STOP review** |
| 7 | ARP/IP later |

---

## Next: E1000_PACKET_BUFFER_UC_ALIAS_PROOF_V1

## Commit
```bash
git add docs/handoff/E1000_PACKET_BUFFER_ALLOCATION_PLAN_V1.md
git commit -m "docs(net): packet buffer allocation plan V1"
```
