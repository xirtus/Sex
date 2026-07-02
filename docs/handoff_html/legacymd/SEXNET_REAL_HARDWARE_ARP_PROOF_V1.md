# SEXNET_REAL_HARDWARE_ARP_PROOF_V1

Date: 2026-05-19
Branch: master
Phase: N (Real Hardware Audit)
Task: 71

## Goal
Prove or honestly SKIP real hardware ARP request/reply.

## Prerequisites

This proof requires:
1. A supported NIC (e1000/e1000e compatible) — NOT MET
2. BAR0 MMIO readable and ring-programmable — NOT MET
3. RX/TX stop review passing — NOT MET (STOP FIRST)
4. Link up — NOT MET (NO-CARRIER)
5. Real hardware boot log with ARP markers — NOT AVAILABLE

## Decision: SKIP

**Reason:** `no_supported_nic_no_real_boot_log_rx_tx_stop_first`

### Why SKIP (Not STOP FIRST)

- ARP proof itself is not unsafe — it's simply impossible without the
  prerequisites.
- STOP FIRST applies to the RX/TX review (Task 70), not to the ARP
  proof itself.
- SKIP is honest: we lack the hardware, the boot log, and the safe
  RX/TX path needed to attempt ARP.

## Required SKIP Marker

```
[sexnet.real_hw.arp.proof.skip] reason=no_supported_nic_rx_tx_stop_first ok=1
```

## What ARP PASS Would Look Like

If a supported NIC and real boot log existed:
```
[sexnet.real_hw.arp.request.tx] tx_dd=1 ok=1
[sexnet.real_hw.arp.reply.rx] rx_arp=1 ok=1
[sexnet.real_hw.arp.proof.done] ok=1
```

These markers are NOT present in any log.

## No Unsafe Actions Taken

- No ARP frames sent to unsupported NIC.
- No ARP cache populated from real hardware.
- No gateway MAC resolved on unsupported hardware.
- QEMU ARP proof (Phase A/B) remains valid and unaffected.

## Proof Commands

Not applicable (no real hardware boot with supported NIC available).
