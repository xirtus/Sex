# DMA_STATIC_RING_ALLOCATION_PROOF_V1

**Status:** PASS IMPLEMENTED — 137/137 gates, 0 faults.
**Date:** 2026-05-17

---

## Result

Static DMA ring allocation proved. `allocated=1`, `rings_enabled=0`, `mmio_writes=0`, `packets=0`.

---

## Ring Allocation Table

| Ring | Phys Addr        | Virt Addr (HHDM)    | Bytes | Align | Cache Policy |
|------|------------------|---------------------|-------|-------|--------------|
| RX   | 0x1F880000       | 0xFFFF80001F880000  | 4096  | 4096  | UC (intent)  |
| TX   | 0x102AA000       | 0xFFFF8000102AA000  | 4096  | 4096  | UC (intent)  |

- Descriptor count: 256 per ring (256 × 16 B = 4096 B = one 4K page)
- Packet buffers: deferred to V2 (not allocated in V1)
- Both rings zeroed via `core::ptr::write_bytes` at HHDM virt address

---

## Allocator Path

`crate::memory::allocator::alloc_frame()` → `GLOBAL_ALLOCATOR.alloc(0)` (order-0 = 4K).

- No allocator changes made. Existing buddy allocator used as-is.
- `alloc_frame()` is safe to call after `memory::manager::init()`.
- `enumerate_bus()` runs inside `hal::init_advanced()` which is called after `memory::manager::init()` — allocator initialized, GLOBAL_ALLOCATOR ready.
- Order-0 allocation guarantees 4K alignment by buddy invariant.

---

## UC/Cache Policy Proof

- Descriptor rings will use `PageTableFlags::NO_CACHE | PageTableFlags::WRITE_THROUGH` at driver attach.
- This flag combination already exists in `kernel/src/syscalls/mod.rs:360-366` for MMIO BAR mapping.
- V1 proof: rings allocated and zeroed via WB HHDM identity mapping (safe before device DMA).
- `cache=UC` in markers = intended policy declaration; actual UC PTE deferred to driver attach.
- No device DMA is occurring in V1, so WB access for zeroing is safe.

---

## Proof Markers (from boot log)

```
[dma.static.ring.alloc] rx_bytes=4096 tx_bytes=4096 rx_align=4096 tx_align=4096 cache=UC allocated=1 ok=1 reason=alloc_frame_order0
[e1000.ring.phys] rx_phys=0x000000001F880000 tx_phys=0x00000000102AA000 rx_virt=0xFFFF80001F880000 tx_virt=0xFFFF8000102AA000 ok=1 reason=hhdm_identity
[e1000.ring.truth] allocated=1 rings_enabled=0 dma=0 mmio_writes=0 irq=0 packets=0 ok=1 reason=static_ring_allocation_proof
[browser.nic.truth] slot_net_grant=0 network=0 fetched=0 dns=0 tcp=0 http=0 tls=0 ok=1 reason=no_network_capability
[dma.static.ring.allocation.proof.done] ok=1 allocated=1 packets=0
```

---

## Files Changed

| File | Change |
|------|--------|
| `kernel/src/hal/pci.rs` | +35 lines: ring allocation inside e1000 detection block |
| `scripts/daily_driver_master_gate.sh` | +5 gate declarations, +45 lines check logic, +5 ALL_GATES entries |
| `docs/handoff/DMA_STATIC_RING_ALLOCATION_PROOF_V1.md` | new |

---

## Proof Result

- **137/137 PASS** (was 132)
- **0 faults**
- 5 new gates: `dma_static_ring_alloc`, `e1000_ring_phys`, `e1000_ring_truth`, `browser_nic_truth`, `dma_ring_alloc_proof_done`
- `e1000_ring_alloc` gate updated to also recognize new proof marker

---

## Fault Count

0

---

## Blockers for V2

| Blocker | Reason |
|---------|--------|
| Packet buffers (8 × 2K) | Not allocated in V1. Deferred — requires confirming lifetime and no device use. |
| MMIO ring base writes (RDBAL/TDBAL) | **STOP FIRST** — first MMIO write to device register. Requires driver attach decision. |
| RX/TX enable (RCTL/TCTL) | **STOP FIRST** — enables device DMA. Full driver review required. |
| UC PTE remap via `map_physical_range` | Must happen before any device DMA. Use `NO_CACHE|WRITE_THROUGH` flags. |
| Packet send/recv | **STOP FIRST** — full packet pipeline review required. |

---

## Commit

```bash
git add kernel/src/hal/pci.rs scripts/daily_driver_master_gate.sh docs/handoff/DMA_STATIC_RING_ALLOCATION_PROOF_V1.md
git commit -m "feat(dma): static e1000 ring allocation proof V1"
```
