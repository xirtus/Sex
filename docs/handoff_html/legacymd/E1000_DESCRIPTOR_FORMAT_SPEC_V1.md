# E1000_DESCRIPTOR_FORMAT_SPEC_V1

**Status:** PASS REVIEW ONLY — Docs-only spec.
**Date:** 2026-05-16

---

## RX Descriptor (16 bytes, little-endian)

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| 0 | 8 | buffer_addr | Physical address of packet buffer |
| 8 | 2 | length | Buffer size / received length |
| 10 | 2 | checksum | Hardware checksum |
| 12 | 1 | status | DD (bit 0) = descriptor done, EOP (bit 1) |
| 13 | 1 | errors | Error flags |
| 14 | 2 | special | VLAN tag |

## TX Descriptor (16 bytes, little-endian)

| Offset | Size | Field | Notes |
|--------|------|-------|-------|
| 0 | 8 | buffer_addr | Physical address of packet buffer |
| 8 | 2 | length | Packet length to send |
| 10 | 1 | cso | Checksum offset |
| 11 | 1 | cmd | EOP (bit 0), IFCS (bit 1), RS (bit 3) |
| 12 | 1 | status | DD (bit 0) |
| 13 | 1 | css | Checksum start |
| 14 | 2 | special | VLAN tag |

## Rust Representation

```rust
#[repr(C, align(16))]
struct RxDescriptor {
    buffer_addr: u64,  // physical
    length:      u16,
    checksum:    u16,
    status:      u8,
    errors:      u8,
    special:     u16,
}
// static_assert!(size_of::<RxDescriptor>() == 16);

#[repr(C, align(16))]
struct TxDescriptor {
    buffer_addr: u64,  // physical
    length:      u16,
    cso:         u8,
    cmd:         u8,
    status:      u8,
    css:         u8,
    special:     u16,
}
```

Access via `read_volatile`/`write_volatile` on raw pointers. No Rust references into DMA memory.

---

## Ring Metadata

| Field | Value |
|-------|-------|
| descriptor_count | 256 |
| descriptor_bytes | 4096 |
| rx_phys/rx_virt | boot-allocated |
| tx_phys/tx_virt | boot-allocated |
| rings_enabled | 0 (deferred to MMIO write phase) |
| head/tail | 0 (cached, updated after each batch) |

---

## Next: E1000_RING_ALLOCATION_STUB_V1

## Commit
```bash
git add docs/handoff/E1000_DESCRIPTOR_FORMAT_SPEC_V1.md
git commit -m "docs(net): e1000 descriptor format spec V1"
```
