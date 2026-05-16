# E1000_DESCRIPTOR_LINK_PLAN_V1

**Status:** PASS REVIEW ONLY — Docs-only plan.
**Date:** 2026-05-17
**Gates:** 143/143 baseline (E1000_PACKET_BUFFER_UC_ALIAS_PROOF_V1).
**Scope:** Plan only. No implementation. No descriptor writes. No MMIO writes. No RX/TX enable. No packets.

---

## 1. Context

### 1.1 Current State (after E1000_PACKET_BUFFER_UC_ALIAS_PROOF_V1)

| Truth Field          | Value | Meaning                                     |
|----------------------|-------|---------------------------------------------|
| descriptor_linked    | 0     | No descriptors point at packet buffers yet  |
| device_visible       | 0     | Device has no knowledge of rings or buffers |
| mmio_writes          | 0     | No BAR/register writes                      |
| dma                  | 0     | DMA engine not enabled                      |
| rings_enabled        | 0     | RCTL.EN=0, TCTL.EN=0                        |
| packets              | 0     | No packet send or receive                   |
| browser_network      | 0     | No Browser network capability               |

### 1.2 Memory Inventory

| Resource           | Phys Base     | UC Alias Base          | Count  | Size      |
|--------------------|---------------|------------------------|--------|-----------|
| RX descriptor ring | allocated     | 0xFFFF9000XXXXXXXX     | 1 page | 4096      |
| TX descriptor ring | allocated     | 0xFFFF9000XXXXXXXX     | 1 page | 4096      |
| RX packet buffers  | 8 pages       | 0xFFFF9000XXXXXXXX × 8 | 8      | 2048 each |
| TX packet buffers  | 8 pages       | 0xFFFF9000XXXXXXXX × 8 | 8      | 2048 each |

All UC aliased with NO_CACHE | WRITE_THROUGH. All zeroed via HHDM.

### 1.3 Descriptor Format (from E1000_DESCRIPTOR_FORMAT_SPEC_V1)

**RX Descriptor** (16 bytes, little-endian, 256 per ring):
| Offset | Size | Field       | Initial Value |
|--------|------|-------------|---------------|
| 0      | 8    | buffer_addr | packet buffer physical address |
| 8      | 2    | length      | 0 (device fills on packet arrival) |
| 10     | 2    | checksum    | 0 (device fills) |
| 12     | 1    | status      | 0 (driver owns — DD=0) |
| 13     | 1    | errors      | 0 |
| 14     | 2    | special     | 0 |

**TX Descriptor** (16 bytes, little-endian, 256 per ring):
| Offset | Size | Field       | Initial Value |
|--------|------|-------------|---------------|
| 0      | 8    | buffer_addr | packet buffer physical address |
| 8      | 2    | length      | 0 (no packet data) |
| 10     | 1    | cso         | 0 |
| 11     | 1    | cmd         | 0 (no EOP/IFCS/RS) |
| 12     | 1    | status      | 0 (driver owns — DD=0) |
| 13     | 1    | css         | 0 |
| 14     | 2    | special     | 0 |

---

## 2. RX Descriptor Linking Plan

### 2.1 What Gets Linked

8 RX descriptors (ring indices 0–7) linked to 8 RX packet buffers (buffers 0–7).

### 2.2 Per-Descriptor Write Sequence

For each RX descriptor `i` (0 ≤ i ≤ 7), write the following via `write_volatile` to UC alias ring memory:

```
// Computed addresses
rx_desc_base = RX_RING_UC_ALIAS        // UC alias VA of RX ring page
desc_offset  = i * 16                   // 16 bytes per descriptor
desc_ptr     = rx_desc_base + desc_offset

// Field writes (all write_volatile, raw pointer, u64/u16/u8 aligned)
write_volatile((desc_ptr + 0)  as *mut u64, rx_buffer_phys[i])   // buffer_addr
write_volatile((desc_ptr + 8)  as *mut u16, 0u16)                // length
write_volatile((desc_ptr + 10) as *mut u16, 0u16)                // checksum
write_volatile((desc_ptr + 12) as *mut u8,  0u8)                 // status
write_volatile((desc_ptr + 13) as *mut u8,  0u8)                 // errors
write_volatile((desc_ptr + 14) as *mut u16, 0u16)                // special
```

### 2.3 What Does NOT Happen

- **No ring base MMIO writes** (RDBAL/RDBAH unchanged). Device unaware ring exists.
- **No ring length MMIO writes** (RDLEN unchanged).
- **No tail pointer update** (RDT=0). Device owns no descriptors.
- **No RX enable** (RCTL.EN=0).
- **No ownership transfer.** DD=0 means driver-owned from the device's perspective (if it could see the ring).

### 2.4 Why Safe Before MMIO Writes

The e1000 device polls descriptors only after:
1. RDBAL/RDBAH set to ring base physical address
2. RDLEN set to ring size
3. RDT advanced past descriptors the driver gives to the device
4. RCTL.EN=1

Until step 4, the device does not DMA-read descriptor ring memory. Descriptor writes before MMIO configuration are pure memory writes — invisible to the device.

---

## 3. TX Descriptor Linking Plan

### 3.1 What Gets Linked

8 TX descriptors (ring indices 0–7) linked to 8 TX packet buffers (buffers 8–15).

### 3.2 Per-Descriptor Write Sequence

For each TX descriptor `i` (0 ≤ i ≤ 7), write via `write_volatile` to UC alias ring memory:

```
// Computed addresses
tx_desc_base = TX_RING_UC_ALIAS        // UC alias VA of TX ring page
desc_offset  = i * 16
desc_ptr     = tx_desc_base + desc_offset

write_volatile((desc_ptr + 0)  as *mut u64, tx_buffer_phys[i])   // buffer_addr
write_volatile((desc_ptr + 8)  as *mut u16, 0u16)                // length
write_volatile((desc_ptr + 10) as *mut u8,  0u8)                 // cso
write_volatile((desc_ptr + 11) as *mut u8,  0u8)                 // cmd
write_volatile((desc_ptr + 12) as *mut u8,  0u8)                 // status
write_volatile((desc_ptr + 13) as *mut u8,  0u8)                 // css
write_volatile((desc_ptr + 14) as *mut u16, 0u16)                // special
```

### 3.3 What Does NOT Happen

- No TDBAL/TDBAH writes
- No TDLEN write
- No TDT update (TDT=0)
- No TX enable (TCTL.EN=0)
- No packet data in buffers
- No CMD bits set (no EOP, no RS, no IFCS)

### 3.4 TX Descriptor CMD Field — Deliberately Zero

| CMD Bit | Name | Set? | Why Not |
|---------|------|------|---------|
| 0       | EOP  | 0    | No packet data — end-of-packet meaningless |
| 1       | IFCS | 0    | No Ethernet frame to CRC |
| 3       | RS   | 0    | No report-status needed — no MMIO enabled |
| 4       | RPS  | 0    | Same as above |
| 7       | IDE  | 0    | No interrupt delay enable |
| others  | —    | 0    | Reserved / context descriptors |

---

## 4. Write Safety Model

### 4.1 Golden Rules

1. **All descriptor writes go to UC alias VA** (`0xFFFF9000XXXXXXXX`), not HHDM (`0xFFFF8000XXXXXXXX`).
2. **`write_volatile` only** on raw pointers. No `&mut` references into DMA memory.
3. **No `#[repr(C)]` struct reference casts.** Write individual fields at known offsets.
4. **Sequential write order matters less before MMIO**, but still do buffer_addr first, then zeros.
5. **No read-modify-write** on descriptor fields. Write known values directly.
6. **TLB already flushed** for UC alias pages (from UC alias proof).

### 4.2 Why Not Use repr(C) Struct Pointers

```rust
// DANGER — forbidden:
let desc = &mut *(desc_ptr as *mut RxDescriptor);
desc.buffer_addr = phys;  // Rust reference into DMA memory = UB
```

The `write_volatile` approach avoids:
- Stacked borrows / aliasing violations on DMA memory
- Compiler reordering across volatile device memory
- Accidental read-modify-write of adjacent descriptor fields
- `unsafe` `&mut` across page boundaries

### 4.3 Pointer Arithmetic Safety

All descriptor writes are within a single 4096-byte UC-aliased page:
- Ring base: page-aligned (4K)
- Descriptors 0–7: offsets 0x00–0x7F (first 128 bytes)
- Remaining descriptors 8–255: untouched (offsets 0x80–0xFFF)
- No out-of-bounds access possible

### 4.4 No UB From Uninitialized Descriptor Slots

Descriptors 8–255 remain zeroed (ring was zeroed at allocation). If the device ever reads them (after MMIO enable), it sees:
- buffer_addr=0 → invalid physical address
- status=0 → driver-owned
- Device skips these descriptors because tail never advances past 7.

---

## 5. Metadata After Link Implementation

| Field               | Value | Meaning |
|---------------------|-------|---------|
| rx_linked_count     | 8     | First 8 RX descriptors linked |
| tx_linked_count     | 8     | First 8 TX descriptors linked |
| descriptor_linked   | 1     | Proof gate passes |
| device_visible      | 0     | No MMIO ring base writes yet |
| rings_enabled       | 0     | RCTL.EN=0, TCTL.EN=0 |
| dma                 | 0     | No DMA engine active |
| mmio_writes         | 0     | No register writes |
| packets             | 0     | No packet data |

---

## 6. Future Gate Markers

```
// Phase 1 — Descriptor link proof (implementation)
[e1000.rx.desc.link] idx_start=0 idx_end=7 count=8 phys_addr=0xNNNNNNNNNNNNNNNN ok=N reason=...
[e1000.tx.desc.link] idx_start=0 idx_end=7 count=8 phys_addr=0xNNNNNNNNNNNNNNNN ok=N reason=...
[e1000.desc.link.truth] rx_linked=8 tx_linked=8 descriptor_linked=1 device_visible=0 mmio_writes=0 dma=0 rings_enabled=0 packets=0 ok=N reason=...
[e1000.desc.link.done] ok=N rx_linked=8 tx_linked=8 packets=0

// Phase 2 — Descriptor readback proof
[e1000.desc.readback] idx=N role=NAME buffer_addr=0xNN expected=0xNN match=N ok=N reason=...

// Phase 3 — MMIO ring base write plan (docs only)
[e1000.mmio.ring.base.plan] ...

// Phase 4 — RX ring base write proof (MMIO writes start here)
[e1000.rx.ring.base.write] ...

// Phase 5 — RX enable plan (docs only)
[e1000.rx.enable.plan] ...

// Phase 6 — TX test-frame STOP review (review only, no send)
[e1000.tx.packet.stop_review] ...

// Phase 7 — ARP/IP plan (future)
```

---

## 7. Phase Ladder

| Phase | Name                                | Type     | Descriptor Writes | MMIO Writes | Packets |
|-------|-------------------------------------|----------|-------------------|-------------|---------|
| 0     | **This plan**                       | docs     | No                | No          | No      |
| 1     | E1000_DESCRIPTOR_LINK_PROOF_V1     | impl     | Yes (8 RX + 8 TX)| No          | No      |
| 2     | E1000_DESCRIPTOR_READBACK_PROOF_V1 | impl     | Read-only verify  | No          | No      |
| 3     | E1000_MMIO_RING_BASE_PLAN_V1       | docs     | No                | No          | No      |
| 4     | E1000_MMIO_RING_BASE_PROOF_V1      | impl     | Already done      | Yes (ring base) | No   |
| 5     | E1000_RX_ENABLE_PLAN_V1            | docs     | No                | No          | No      |
| 6     | E1000_TX_PACKET_STOP_REVIEW        | review   | Review only       | Review only | Review  |
| 7     | E1000_ARP_IP_PLAN_V1               | docs     | —                 | —           | Future  |

---

## 8. STOP FIRST Boundaries

The following actions are **explicitly blocked** in Phase 1 (and remain blocked through Phase 4):

| Boundary                     | Why Blocked                                          |
|------------------------------|------------------------------------------------------|
| MMIO ring base writes        | Would make device aware of descriptor rings          |
| RX/TX enable                 | Would start DMA engine                               |
| Tail pointer update          | Would transfer descriptor ownership to device        |
| CMD.EOP in TX descriptors    | Would claim a complete frame exists                  |
| Packet data in buffers        | Would be transmitted if TX enabled                   |
| Browser SLOT_NET grant       | Would expose NIC to Browser before kernel ready      |
| Interrupt enable             | Would require ISR that doesn't exist yet             |
| Context descriptors          | Advanced TX offload — not needed for basic link      |
| receive descriptor multiple  | 256-descriptor ring for now; 1K is 82540EM max        |
| ARP/IP packet construction   | Premature — no MMIO config, no RX/TX enable          |

---

## 9. Descriptor UB / Packed Reference Risks

### 9.1 Why No struct Pointers

The e1000 descriptor layout uses unaligned fields (e.g., `length` at offset 8 but is only `u16`, not `u64`). Rust's `#[repr(C, align(16))]` ensures 16-byte alignment of the struct start, but:

- Field-level unaligned access on x86_64 is permitted but `write_volatile` at field offsets is clearer about intent
- No risk of compiler inserting padding between fields
- No risk of MIRI flagging UB on `&mut` into MMIO/DMA memory
- Explicit `write_volatile` is auditable — each offset maps to a known datasheet field

### 9.2 Atomicity

Individual `write_volatile` for u64, u16, u8 are naturally atomic on x86_64 (aligned). Since the device cannot read the ring yet (no MMIO config), write ordering is irrelevant for Phase 1.

---

## 10. Recommendations

### 10.1 Implementation Approach (for E1000_DESCRIPTOR_LINK_PROOF_V1)

1. In `kernel/src/hal/pci.rs`, after packet buffer UC alias proof block
2. Store packet buffer physical addresses from `pkt_pages[]` array
3. Compute UC alias addresses for RX and TX ring bases (already in scope)
4. Loop 0..8: write_volatile buffer_addr and zero remaining fields for RX descriptors
5. Loop 0..8: same for TX descriptors
6. Emit `[e1000.rx.desc.link]`, `[e1000.tx.desc.link]`, `[e1000.desc.link.truth]`, `[e1000.desc.link.done]`
7. Add 4 gate entries in daily_driver_master_gate.sh
8. No MMIO writes. No RX/TX enable. No packets.

### 10.2 Next Prompt

**E1000_DESCRIPTOR_LINK_PROOF_V1** — Implement descriptor linking per this plan.

---

## 11. Commit

```bash
git add docs/handoff/E1000_DESCRIPTOR_LINK_PLAN_V1.md
git commit -m "docs(net): e1000 descriptor link plan V1"
```
