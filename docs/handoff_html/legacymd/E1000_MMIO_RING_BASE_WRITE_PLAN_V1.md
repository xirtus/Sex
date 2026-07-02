# E1000_MMIO_RING_BASE_WRITE_PLAN_V1

**Status:** PASS REVIEW ONLY — Docs-only plan.
**Date:** 2026-05-17
**Gates:** 151/151 baseline (E1000_DESCRIPTOR_READBACK_PROOF_V1).
**Scope:** Plan only. No implementation. No MMIO writes. No RX/TX enable. No packets.

---

## 1. Context

### 1.1 Current State

| Truth Field              | Value | Meaning                                        |
|--------------------------|-------|------------------------------------------------|
| descriptor_linked        | 1     | 8 RX + 8 TX descriptors linked to buffers      |
| descriptor_readback      | 1     | Buffer_addr writes verified via read_volatile  |
| device_visible           | 0     | Device unaware of rings/buffers                |
| mmio_writes              | 0     | No register writes                             |
| dma                      | 0     | DMA engine not active                          |
| rings_enabled            | 0     | RCTL.EN=0, TCTL.EN=0                           |
| rx_enabled               | 0     | RX not enabled                                 |
| tx_enabled               | 0     | TX not enabled                                 |
| packets                  | 0     | No TX or RX data                               |
| browser_network          | 0     | No Browser net capability                      |

### 1.2 Memory Inventory (all proven)

| Resource             | Phys Example  | UC Alias Example       | Purpose                    |
|----------------------|---------------|------------------------|----------------------------|
| RX ring page         | allocated     | 0xFFFF9000XXXXXXXX     | 256 RX descriptors         |
| TX ring page         | allocated     | 0xFFFF9000XXXXXXXX     | 256 TX descriptors         |
| RX buffers (8×2K)    | 4 pages       | 0xFFFF9000XXXXXXXX ×4  | Packet receive buffers     |
| TX buffers (8×2K)    | 4 pages       | 0xFFFF9000XXXXXXXX ×4  | Packet transmit buffers    |

All rings/buffers 4K-aligned, zeroed, UC-aliased with NO_CACHE | WRITE_THROUGH.

### 1.3 BAR0 MMIO Base

```
bar0_phys = dev.get_bar(0)             // from PCI config space (mem BAR)
mmio_base = 0xFFFF_8000_0000_0000 + bar0_phys  // HHDM VA — existing read pattern
```

---

## 2. Register Write Objective

Write ring base/length/head/tail registers to inform the e1000 device where the descriptor rings reside in physical memory. **Do not enable RX or TX.** The device will know the ring locations but will not DMA-read descriptors until both tail is advanced AND the enable bit is set.

### 2.1 Registers to Write

| Phase           | Registers            | Count | Purpose                                |
|-----------------|----------------------|-------|----------------------------------------|
| Ring base write | RDBAL, RDBAH, RDLEN  | 3     | Tell device where RX ring lives        |
|                 | TDBAL, TDBAH, TDLEN  | 3     | Tell device where TX ring lives        |
| Head/tail init  | RDH, RDT             | 2     | Initialize RX head/tail to 0           |
|                 | TDH, TDT             | 2     | Initialize TX head/tail to 0           |
| **Total**       |                       | **10** | 32-bit writes                          |

### 2.2 Registers NOT Written

| Register | Reason                                                      |
|----------|-------------------------------------------------------------|
| RCTL     | Contains EN bit (bit 1) — must stay 0                      |
| TCTL     | Contains EN bit (bit 1) — must stay 0                      |
| ICR/IAM  | Interrupt registers — no IRQ enable                        |
| RXCSUM   | RX checksum offload — deferred                              |
| Any other| Only ring base/len/head/tail touched in this phase         |

---

## 3. Register Map (82540EM / e1000)

### 3.1 RX Ring Registers

| Register | Offset  | Size  | Description                             | Write Value                  |
|----------|---------|-------|-----------------------------------------|------------------------------|
| RDBAL    | 0x2800  | u32   | RX Descriptor Base Address Low          | `rx_ring_phys & 0xFFFFFFFF`  |
| RDBAH    | 0x2804  | u32   | RX Descriptor Base Address High         | `(rx_ring_phys >> 32) as u32`|
| RDLEN    | 0x2808  | u32   | RX Descriptor Ring Length (bytes)       | `4096`                       |
| RDH      | 0x2810  | u32   | RX Descriptor Head (driver read index)  | `0`                          |
| RDT      | 0x2818  | u32   | RX Descriptor Tail (device read limit)  | `0`                          |

**RDLEN constraint**: Must be 128-byte aligned. 4096 = 0x1000 ✓  
**RDT=0 interpretation**: Device owns zero RX descriptors (head == tail → empty ring).  
**RDH=0 interpretation**: Driver will next check descriptor 0 when reading.

### 3.2 TX Ring Registers

| Register | Offset  | Size  | Description                             | Write Value                  |
|----------|---------|-------|-----------------------------------------|------------------------------|
| TDBAL    | 0x3800  | u32   | TX Descriptor Base Address Low          | `tx_ring_phys & 0xFFFFFFFF`  |
| TDBAH    | 0x3804  | u32   | TX Descriptor Base Address High         | `(tx_ring_phys >> 32) as u32`|
| TDLEN    | 0x3808  | u32   | TX Descriptor Ring Length (bytes)       | `4096`                       |
| TDH      | 0x3810  | u32   | TX Descriptor Head (driver read index)  | `0`                          |
| TDT      | 0x3818  | u32   | TX Descriptor Tail (device read limit)  | `0`                          |

**TDLEN constraint**: Same as RDLEN — 128-byte aligned. 4096 ✓  
**TDT=0 interpretation**: Device owns zero TX descriptors. No packets will be sent.

### 3.3 Why RDT=0 and TDT=0 Are Safe

The e1000 device owns descriptors in the range `[RDH, RDT)` with modular wrap:
- When RDH == RDT → zero descriptors owned → device does nothing
- Even if RCTL.EN=1 (which it won't be), the ring appears empty
- The 8 pre-linked RX descriptors at indices 0–7 have `status=0` (DD=0)
- They are "driver-owned" regardless of head/tail values
- No DMA activity occurs with empty rings

When RX is later enabled (future phase):
1. Driver sets RDT=8 → device owns descriptors 0–7
2. Driver sets RCTL.EN=1 → device begins polling for packets
3. On packet arrival, device writes to buffer[descriptor].buffer_addr, sets DD=1

---

## 4. MMIO Write API

### 4.1 Access Pattern

```rust
// All writes go to HHDM-virtualized BAR0 — same pattern as existing reads
let mmio_base = 0xFFFF_8000_0000_0000u64 + bar0_phys;

unsafe {
    // RX ring base
    core::ptr::write_volatile((mmio_base + 0x2800) as *mut u32, (rx_phys & 0xFFFF_FFFF) as u32);
    core::ptr::write_volatile((mmio_base + 0x2804) as *mut u32, (rx_phys >> 32) as u32);
    core::ptr::write_volatile((mmio_base + 0x2808) as *mut u32, 4096u32);
    core::ptr::write_volatile((mmio_base + 0x2810) as *mut u32, 0u32);
    core::ptr::write_volatile((mmio_base + 0x2818) as *mut u32, 0u32);

    // TX ring base
    core::ptr::write_volatile((mmio_base + 0x3800) as *mut u32, (tx_phys & 0xFFFF_FFFF) as u32);
    core::ptr::write_volatile((mmio_base + 0x3804) as *mut u32, (tx_phys >> 32) as u32);
    core::ptr::write_volatile((mmio_base + 0x3808) as *mut u32, 4096u32);
    core::ptr::write_volatile((mmio_base + 0x3810) as *mut u32, 0u32);
    core::ptr::write_volatile((mmio_base + 0x3818) as *mut u32, 0u32);
}
```

### 4.2 Cache / Memory Ordering

- HHDM mapping is **WB (Write-Back)** — writes may be buffered in CPU cache
- `write_volatile` prevents compiler reordering but does **not** guarantee write-combining flush
- For safety on x86_64: a `mfence` or `sfence` after all MMIO writes ensures device visibility
- The plan should record whether a fence is needed (likely not critical before enable)
- Future phase should consider UC remap of BAR0 for MMIO writes

### 4.3 Write Ordering Within a Ring

For each ring, the recommended order is:
1. RDBAL → low 32 bits of base address
2. RDBAH → high 32 bits (device latches base on RDBAH write)
3. RDLEN → ring size
4. RDH → head (0)
5. RDT → tail (0)

The e1000 datasheet specifies that RDBAH write triggers the device to latch both RDBAL and RDBAH. Similarly for TX.

### 4.4 Safety Model

| Property             | Guarantee                                             |
|----------------------|-------------------------------------------------------|
| Atomicity            | Each 32-bit write is naturally atomic on x86_64       |
| Alignment            | All register offsets are 4-byte aligned               |
| No partial writes    | Each register written as single u32                   |
| No read-modify-write | Known values written directly                         |
| RCTL/TCTL untouched  | EN bits remain 0 — no DMA activity                    |
| Tail=0               | Device owns zero descriptors regardless of enable     |
| Browser isolation    | Browser never touches BAR0 MMIO                       |

---

## 5. Safety Invariants

### 5.1 Pre-conditions (must be true before MMIO ring base write)

| # | Invariant | Proven In |
|---|-----------|-----------|
| 1 | RX/TX ring pages allocated, 4K aligned | DMA_STATIC_RING_ALLOCATION_PROOF_V1 |
| 2 | RX/TX ring pages UC-aliased | DMA_UC_ALIAS_REMAP_PROOF_V1 |
| 3 | RX/TX descriptors linked to buffer phys addresses | E1000_DESCRIPTOR_LINK_PROOF_V1 |
| 4 | Descriptor buffer_addr values read back and verified | E1000_DESCRIPTOR_READBACK_PROOF_V1 |
| 5 | Packet buffers allocated, zeroed, UC-aliased | E1000_PACKET_BUFFER_UC_ALIAS_PROOF_V1 |
| 6 | BAR0 MMIO base accessible (read-only proven) | E1000_MAC_READ_PROOF_V1 |

### 5.2 Post-conditions (after MMIO ring base write)

| # | Invariant | Check |
|---|-----------|-------|
| 1 | RDBAL/RDBAH = rx_ring_phys | Read back and compare |
| 2 | TDBAL/TDBAH = tx_ring_phys | Read back and compare |
| 3 | RDLEN = TDLEN = 4096 | Read back and compare |
| 4 | RDH = TDH = 0 | Read back and compare |
| 5 | RDT = TDT = 0 | Read back and compare |
| 6 | RCTL unchanged (EN=0) | Read back and verify bit 1 = 0 |
| 7 | TCTL unchanged (EN=0) | Read back and verify bit 1 = 0 |
| 8 | No faults | Fault scan |
| 9 | Browser network=0 | Gate check |

### 5.3 What Remains False After Ring Base Write

| Field            | Still Value | Why                                              |
|------------------|-------------|--------------------------------------------------|
| device_visible   | 1           | Device now knows ring locations                  |
| rings_enabled    | 0           | RCTL.EN=0, TCTL.EN=0                             |
| rx_enabled       | 0           | RCTL.EN=0                                        |
| tx_enabled       | 0           | TCTL.EN=0                                        |
| dma              | 0           | No descriptors owned (tail=0) + not enabled      |
| packets          | 0           | No RX or TX data                                 |
| irq              | 0           | No interrupt registers touched                   |

Note: `device_visible` transitions from 0→1 because the device can now locate the rings in physical memory. However, without tail advancement or enable bits, the device does not read/write ring memory.

---

## 6. Future Proof Markers

```
// Phase 4 implementation markers
[e1000.mmio.ring.base.write] offset=0x2800..0x2818 count=5 ok=N reason=...
[e1000.mmio.ring.base.write] offset=0x3800..0x3818 count=5 ok=N reason=...
[e1000.mmio.ring.base.readback] rx_base_match=1 tx_base_match=1 len_match=1 head_tail_zero=1 ok=N reason=...
[e1000.mmio.ring.base.truth] device_visible=1 rings_enabled=0 rx_enabled=0 tx_enabled=0 mmio_writes=10 dma=0 packets=0 ok=N reason=...
[e1000.mmio.ring.base.proof.done] ok=N writes=10 readback=1 packets=0

// Control register safety check
[e1000.rctl.safety] offset=0x0100 en_bit=0 ok=N reason=...
[e1000.tctl.safety] offset=0x0400 en_bit=0 ok=N reason=...
```

---

## 7. Phase Ladder

| Phase | Name                                     | Type   | MMIO Writes | MMIO What          | RX/TX Enable |
|-------|------------------------------------------|--------|-------------|--------------------|--------------|
| 0     | E1000_DESCRIPTOR_LINK_PLAN_V1            | docs   | No          | —                  | No           |
| 1     | E1000_DESCRIPTOR_LINK_PROOF_V1           | impl   | No          | —                  | No           |
| 2     | E1000_DESCRIPTOR_READBACK_PROOF_V1       | impl   | No          | —                  | No           |
| 3     | **This plan**                            | docs   | No          | —                  | No           |
| 4     | E1000_MMIO_RING_BASE_PROOF_V1            | impl   | **Yes**    | Ring base/len/h/t  | **No**       |
| 5     | E1000_RX_ENABLE_PLAN_V1                  | docs   | No          | —                  | No           |
| 6     | E1000_TX_PACKET_STOP_REVIEW              | review | Review      | Review             | Review       |
| 7     | E1000_ARP_IP_PLAN_V1                     | docs   | —           | —                  | Future       |

---

## 8. STOP FIRST Boundaries

| Action                              | Blocked In Phase 4? | Why                                                  |
|-------------------------------------|---------------------|------------------------------------------------------|
| Writing RCTL (0x0100)               | **YES**             | EN bit would start RX DMA                            |
| Writing TCTL (0x0400)               | **YES**             | EN bit would start TX DMA                            |
| Setting RDT > 0                     | **YES**             | Would transfer descriptor ownership to device        |
| Setting TDT > 0                     | **YES**             | Device would read TX descriptors and transmit        |
| Writing CMD.EOP in TX descriptors   | **YES**             | Combined with tail update would transmit packets     |
| Enabling interrupts (ICR/IAM/IMS)   | **YES**             | No ISR exists                                        |
| Writing packet data to TX buffers   | **YES**             | Premature — no ARP/IP stack                          |
| Reading/writing any other registers | **YES**             | Only ring base registers in Phase 4                  |
| Browser SLOT_NET grant              | **YES** (forever)   | Browser never touches NIC                            |

---

## 9. Risk Assessment

| Risk                                   | Severity | Mitigation                                      |
|----------------------------------------|----------|-------------------------------------------------|
| Cache-coherent MMIO write delay (WB)   | Low      | `write_volatile` + optional `mfence`            |
| BAR0 UC remap needed for correctness   | Low      | HHDM reads already work; writes may need audit  |
| Device latches ring base early         | None     | Device only polls descriptors on tail advance   |
| Ring size validation                   | Low      | 4096 is 128-byte aligned, well within 256KB max |
| Descriptor index overflow at wrap      | None     | tail=0 → no descriptors owned → no wrap         |

---

## 10. Next Prompt

**E1000_MMIO_RING_BASE_PROOF_V1** — Implement MMIO writes per this plan.

---

## 11. Commit

```bash
git add docs/handoff/E1000_MMIO_RING_BASE_WRITE_PLAN_V1.md
git commit -m "docs(net): e1000 MMIO ring base write plan V1"
```
