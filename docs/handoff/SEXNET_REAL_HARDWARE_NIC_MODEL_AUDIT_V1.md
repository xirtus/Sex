# SEXNET_REAL_HARDWARE_NIC_MODEL_AUDIT_V1

Date: 2026-05-19
Branch: master
Phase: N (Real Hardware Audit)
Task: 68

## Goal
Audit real hardware NIC model on the build/test host to determine whether
SexNet can safely touch a real NIC, or whether all real hardware proofs
must be SKIPped in favor of the QEMU usernet + e1000 primary path.

## Host Environment

- Machine: Alienware x17 R1 (laptop)
- OS: Linux (zen kernel)
- Tools available: lspci, ip, ethtool, sysfs

## Real NICs Found

### Wired Ethernet
| Field | Value |
|-------|-------|
| PCI address | 0000:3d:00.0 |
| Vendor ID | 0x10EC (Realtek Semiconductor Co., Ltd.) |
| Device ID | 0x3000 (Killer E3000 2.5GbE Controller) |
| Subsystem | 0x1028:0a8f (Dell) |
| Driver (host) | r8169 |
| Interface | enp61s0 |
| Link state | DOWN (NO-CARRIER) |
| BAR0 | 0x84300000 (64-bit, non-prefetchable, 64K) |
| BAR2 | 0x84310000 (64-bit, non-prefetchable, 16K) |

### WiFi
| Field | Value |
|-------|-------|
| PCI address | 0000:3e:00.0 |
| Vendor ID | 0x8086 (Intel) |
| Device ID | 0x2725 (Wi-Fi 6E AX210/AX1675 Typhoon Peak) |
| Driver (host) | iwlwifi |
| Interface | wlp62s0 |
| Link state | UP |
| Supported for SexNet | NO (WiFi not Ethernet; no WiFi driver in SexNet) |

## Classification

**UNSUPPORTED_MODERN_NIC**

### Rationale

1. The only wired Ethernet NIC is a Realtek Killer E3000 (vendor=0x10EC, device=0x3000).
   This is a modern 2.5GbE controller using the r8169 driver on Linux.
   It is NOT e1000, e1000e, or any member of the Intel 8254x/8257x/I219 family.

2. The WiFi NIC (Intel AX210) is a wireless device. SexNet has no WiFi driver,
   no 802.11 stack, and no wireless MAC layer. WiFi is explicitly unsupported.

3. SexNet's current NIC driver (`kernel/src/hal/pci.rs`) assumes e1000/e1000e
   register layouts (BAR0 MMIO, specific register offsets for STATUS, CTRL, RDBAL,
   RDBAH, TDBAL, TDBAH, etc.). The Realtek RTL8125 family has a completely different
   register map and descriptor format.

4. The wired NIC link is DOWN (NO-CARRIER) — even if a driver existed, no physical
   link means no real hardware RX/TX.

## What This Means

- SexNet CANNOT safely touch the real NIC's BAR0/MMIO registers.
- All real hardware BAR mapping, RX/TX descriptor programming, ARP, and ICMP
  proofs are BLOCKED.
- The QEMU usernet + e1000 path remains the primary proven dev/test path.
- No real hardware boot log exists and cannot be acquired for a supported NIC.

## Recommendation

1. **SKIP** all real hardware BAR/RX/TX/ARP/PING proofs in Phase N.
2. **Retain** QEMU source3 as the primary proven networking path.
3. **Do NOT write** any real NIC MMIO registers.
4. **Do NOT attempt** to add a Realtek r8169/r8125 driver in this phase.
5. **Defer** real hardware NIC driver work to a future phase when
   an e1000-compatible NIC is available (e.g., QEMU PCI passthrough
   of an Intel e1000 device, or a desktop with an Intel I219 NIC).

## Future Considerations

If real hardware support becomes a priority:
- Adding an e1000e driver for Intel I219 (vendor=0x8086, device=0x15F3) NICs
  would be the lowest-risk path.
- QEMU PCI passthrough (VFIO) of an e1000 virtual function could provide
  a real-hardware-adjacent test environment.
- A Realtek r8169 driver would require: new register map, new descriptor format,
  new PHY/MDIO access, new reset sequence — essentially a full new NIC driver.

## Required Marker

```
[sexnet.real_hw.nic_model.audit.done] classification=UNSUPPORTED_MODERN_NIC ok=1
```

## Proof Commands

```bash
./scripts/host_real_hw_nic_audit.sh /tmp/sexnet_real_hw_nic_audit.log
```
