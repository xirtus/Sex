# SEXNET_REAL_HARDWARE_BAR_MAP_PROOF_V1

Date: 2026-05-19
Branch: master
Phase: N (Real Hardware Audit)
Task: 69

## Goal
Prove or honestly SKIP real hardware BAR/MMIO mapping safety for SexNet.

## Prerequisites

This proof requires:
1. A supported NIC (e1000/e1000e compatible) physically present.
2. A real hardware boot log with BAR mapping markers.

## Host NIC Classification

Per Task 68 (`SEXNET_REAL_HARDWARE_NIC_MODEL_AUDIT_V1`):
- **Classification:** UNSUPPORTED_MODERN_NIC
- **Wired NIC:** Realtek Killer E3000 (0x10EC:0x3000), r8169 driver
- **WiFi NIC:** Intel AX210 (0x8086:0x2725), iwlwifi driver
- **e1000/e1000e compatible:** NO
- **Wired link:** DOWN (NO-CARRIER)

## Decision: SKIP

**Reason:** `no_supported_nic`

The host has no e1000 or e1000e-compatible NIC. SexNet's MMIO model
(kernel/src/hal/pci.rs) is built around e1000/e1000e register layouts at known
offsets (STATUS=0x0008, CTRL=0x0000, RDBAL=0x2800, RDBAH=0x2804, etc.).
These offsets are specific to the Intel 8254x/8257x/I219 family.

The Realtek RTL8125 (Killer E3000) uses a completely different register map.
Writing e1000 register offsets to a Realtek BAR would be unsafe:
- Unknown register semantics at those offsets
- Potential for unintended side effects (PHY reset, EEPROM write, etc.)
- No documentation of RTL8125 register layout in this project

Additionally, there is no real hardware boot log available because:
- SexOS has never been booted on this physical hardware
- Ventoy/USB boot of SexOS would expose the unsupported NIC to the kernel HAL
- The kernel HAL would attempt e1000 enumeration on a non-e1000 device

## Required SKIP Marker

```
[sexnet.real_hw.bar.proof.skip] reason=no_supported_nic_or_no_real_boot_log ok=1
```

## What Would Be Required for PASS

To achieve PASS on this proof:
1. An e1000/e1000e-compatible NIC must be physically present.
2. SexOS must be booted on real hardware (or via VFIO PCI passthrough).
3. Boot log markers must show:
   - `[sexnet.real_hw.bar.map] vendor=0x8086 device=0xXXXX bar0=... mmio=1 ok=1`
   - `[sexnet.real_hw.bar.readback] reg=STATUS value=... ok=1`
   - `[sexnet.real_hw.bar.proof.done] ok=1`

These markers are NOT present in any log because the prerequisites are not met.

## No Unsafe Actions Taken

- No real NIC MMIO registers were written.
- No unknown BAR offsets were probed.
- No SexOS boot was attempted on unsupported hardware.
- The QEMU e1000 path is completely unaffected by this audit.

## Proof Commands

Not applicable (no real hardware boot possible with supported NIC).
