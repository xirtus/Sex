# SEXNET_REAL_HARDWARE_RX_TX_STOP_REVIEW_V1

Date: 2026-05-19
Branch: master
Phase: N (Real Hardware Audit)
Task: 70

## Goal
Decide whether real hardware RX/TX is safe to attempt on this host.

## Review Questions and Answers

### 1. Is the NIC e1000/e1000e-compatible?
**NO.** The only wired NIC is a Realtek Killer E3000 (0x10EC:0x3000).
This is a modern 2.5GbE controller with a proprietary register map,
completely different from the Intel e1000/e1000e family.

### 2. Is BAR0 MMIO readable?
**UNKNOWN and UNSAFE TO PROBE.**
The Realtek RTL8125 likely has a BAR0 at 0x84300000 (64-bit, 64K),
but the register layout at that BAR is not documented in this project.
Reading at e1000 offsets (e.g., STATUS at +0x0008) could return
garbage values that might be misinterpreted.

### 3. Are RX/TX descriptor formats known?
**NO.** The Realtek RTL8125 uses a different descriptor ring format
from e1000/e1000e. Descriptor fields, ring layout, tail pointer
mechanism, and status bits are all different.

### 4. Is DMA memory safe and mapped?
**NO.** The SexNet DMA model assumes e1000-compatible descriptor
rings. Writing e1000-format descriptors to memory that a Realtek
NIC would DMA from could result in undefined behavior.

### 5. Are interrupts disabled/masked or bounded polling safe?
**UNKNOWN.** The Realtek RTL8125 interrupt mechanism (MSI-X or
legacy INTx) uses different register offsets and mask bits.
Writing to e1000 IMC/IMS/ICR registers at e1000 offsets would
hit unknown register addresses on the Realtek NIC.

### 6. Is link up?
**NO.** `ip link show enp61s0` reports `state DOWN` and `NO-CARRIER`.
Even if the NIC were supported, there is no physical link to send
or receive frames.

### 7. Can ARP be safely attempted?
**NO.** All of the above blockers apply. ARP frame TX requires:
supported NIC, known descriptor format, DMA-safe memory, and link up.
None of these conditions are met.

### 8. What would cause STOP FIRST?
All of the following would trigger STOP FIRST:
- Attempting to write e1000 register offsets to a Realtek BAR
- Creating e1000-format descriptors for a non-e1000 NIC
- Enabling TX/RX on an unsupported NIC
- Probing unknown MMIO regions
- Adding a new NIC driver without audit
- Changing PCI BAR mapping policy for unsupported hardware

### 9. Is real hardware proof blocked by unsupported NIC?
**YES.** Real hardware RX/TX is fully blocked. All Phase N real
hardware tasks (68-72) are STOP FIRST / SKIP.

## Conclusion: STOP FIRST

```
[sexnet.real_hw.rx_tx.stop_review.stop_first]
reason=unsupported_nic_realtek_e3000_not_e1000_compatible
```

### Rationale

The wired NIC is unsupported. Attempting real hardware RX/TX would:
1. Write to unknown MMIO registers (unsafe)
2. Use wrong descriptor format (undefined behavior)
3. Fail due to no link (wasted effort)
4. Potentially hang or fault the host kernel (MMIO to wrong offsets)

SexNet's proven path remains: QEMU usernet + e1000 (or e1000e).
This is the only safe and tested networking path.

### What Would Make This PASS REVIEW

To change this to PASS REVIEW, one of the following would need to be true:
1. An e1000/e1000e-compatible NIC is physically accessible.
2. A VFIO PCI passthrough of an e1000 virtual function is configured.
3. A Realtek r8169/r8125 driver is fully developed, audited, and tested.

None of these conditions currently hold.

## No Unsafe Actions Taken

- No MMIO writes to real NIC.
- No descriptor programming for unsupported hardware.
- No DMA buffer allocation for unsupported NIC.
- No interrupt configuration for unsupported hardware.
- QEMU e1000 path is completely unaffected.

## Proof Commands

Not applicable (STOP FIRST — no real hardware RX/TX attempted or planned).
