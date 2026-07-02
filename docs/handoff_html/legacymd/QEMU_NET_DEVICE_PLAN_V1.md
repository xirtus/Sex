# QEMU_NET_DEVICE_PLAN_V1

**Status:** PASS REVIEW ONLY — Docs-only plan.
**Date:** 2026-05-16
**Gates:** 127/127 baseline.

---

## NIC Comparison

| Device | Complexity | PCI | MMIO | Descriptors | QEMU | Recommendation |
|--------|-----------|-----|------|-------------|------|----------------|
| e1000 | Medium | Yes | Yes | Ring (RX/TX) | `-device e1000` | **Recommended** — well-documented, simple ring model |
| e1000e | High | Yes | Yes | Ring (extended) | `-device e1000e` | Too complex for first bring-up |
| virtio-net | High | Yes | Yes | Vring (complex) | `-device virtio-net-pci` | Virtio spec is complex |
| rtl8139 | Medium | Yes | PIO+MMIO | Simple ring | `-device rtl8139` | Legacy, simpler than e1000 but less documented |
| ne2k_pci | Low | Yes | PIO | Simple | `-device ne2k_pci` | Ancient, may not work with modern QEMU |

## Recommended: **e1000** (Intel 82540EM)

Simplest well-documented PCI NIC for no_std. Descriptor ring model is straightforward.
QEMU: `-device e1000,netdev=n0 -netdev user,id=n0`

---

## Phase Ladder

| Phase | What |
|-------|------|
| 0 | This plan |
| 1 | QEMU command documented (no source changes) |
| 2 | PCI enumerate — find e1000 on bus |
| 3 | Driver attach status — BAR map, no packets |
| 4 | MAC address read + readiness marker |
| 5 | RX/TX ring setup, no external traffic |
| 6 | Send one bounded test frame (STOP review first) |
| 7 | IP/ARP later |
| 8 | TCP/HTTP later |

---

## Ownership

| Component | Role |
|-----------|------|
| kernel/PCI | Device discovery, BAR mapping |
| sexnet | NIC driver status |
| Browser | No direct NIC access |
| Collar | Network grants (future) |
| Mesh | Route visualization (future) |

---

## STOP FIRST Boundaries

- No kernel PCI changes without review
- No interrupt routing without audit
- No DMA/ring allocation without bounds
- No packet send before explicit STOP review
- Browser never gets direct NIC access
- No POSIX socket assumptions

---

## Next: SEXNET_QEMU_NET_DEVICE_AUDIT_V1

## Commit
```bash
git add docs/handoff/QEMU_NET_DEVICE_PLAN_V1.md
git commit -m "docs(net): QEMU net device plan V1"
```
