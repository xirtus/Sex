# SEXNET_QEMU_NET_DEVICE_AUDIT_V1

**Status:** PASS REVIEW ONLY — PCI enumeration exists, e1000 needs class match.
**Date:** 2026-05-16

---

## Key Finding: PCI enumeration is implemented and working

`kernel/src/drivers/pci.rs`:
- PCI config space reads via IO ports 0xCF8/0xCFC ✅
- Bus scan: 0-7, Device scan: 0-31 ✅
- Vendor/Device/Class/Subclass read ✅
- `bootstrap_drivers()` grants PCI caps to PDs ✅

Currently matches: Class 0x01 Subclass 0x08 (NVMe) → sexdrive, Class 0x03 (GPU) → sexdisplay.
e1000 is Class 0x02 Subclass 0x00 (Ethernet) — NOT currently matched.

---

## Device Readiness Table

| Capability | Status |
|-----------|--------|
| PCI enumeration | ✅ `kernel/src/drivers/pci.rs` |
| Config space reads | ✅ IO ports 0xCF8/0xCFC |
| Class/vendor matching | ✅ but only display + NVMe |
| Ethernet class (0x02) | ❌ Not matched |
| MMIO BAR read | ✅ config read exists, BAR access needs driver |
| IRQ routing | ✅ `register_irq_route()` exists |
| DMA/ring allocation | ❌ Not implemented |
| sexnet PCI access | ❌ No PCI capability grant to sexnet |
| QEMU command | ✅ `-device e1000,netdev=n0 -netdev user,id=n0` |

---

## e1000 Visibility

Adding `-device e1000,netdev=n0 -netdev user,id=n0` to QEMU would place the device on the PCI bus at a standard address. The kernel would enumerate it but skip it (no class 0x02 match). To prove visibility, add a class 0x02 match in `bootstrap_drivers()` that logs the device.

---

## Recommended: **A — QEMU_E1000_PCI_ENUM_PROOF_V1**

Add QEMU e1000 device to smoke command. Add kernel class 0x02 match that logs device presence. Grant PCI cap to sexnet. No packets. No driver.

Changes: kernel/src/drivers/pci.rs (class match), kernel/src/init.rs (sexnet PCI cap grant). STOP FIRST before kernel edits.

---

## STOP FIRST Boundaries

| Boundary | Status |
|----------|--------|
| PCI class match addition | ⚠️ Kernel edit — small, localized |
| MMIO BAR mapping | ❌ Deferred |
| IRQ routing | ✅ Already exists |
| DMA/ring | ❌ Not implemented |
| Browser direct NIC | ❌ Blocked |

---

## Next: QEMU_E1000_PCI_ENUM_PROOF_V1

## Commit
```bash
git add docs/handoff/SEXNET_QEMU_NET_DEVICE_AUDIT_V1.md
git commit -m "docs(net): sexnet QEMU net device audit V1"
```
