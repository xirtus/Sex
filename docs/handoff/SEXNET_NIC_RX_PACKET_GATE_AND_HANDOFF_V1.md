# SEXNET_NIC_RX_PACKET_GATE_AND_HANDOFF_V1

## A. Result
Added a new daily-driver gate `sexnet_nic_rx_packet_observe` in `scripts/daily_driver_master_gate.sh` for the proven temporary sexnet RX observe/restore proof.

This gate is TAP-aware and does not hard-fail ordinary non-TAP runs.

## B. Proof command / host preconditions
Host traffic generator (Terminal B, before/during QEMU):

```bash
while true; do
  sudo arping -I tap0 -c 1 -b 10.0.2.15 2>/dev/null || true
  sleep 0.05
done
```

Proof run (Terminal A):

```bash
QEMU_NET_BACKEND=tap \
QEMU_NET_MODEL=e1000e \
QEMU_TAP_IFNAME=tap0 \
ENABLE_QEMU_USERNET_E1000=1 \
./scripts/run_daily_driver_proof.sh /tmp/sexnet_nic_rx_packet_gate_and_handoff_v1.log
```

Scan:

```bash
grep -E "sexnet_nic_rx_packet_observe|sexnet.nic.rx.observe|fault.kill|#PF|#GP|panic|KERNEL PANIC|FINAL:" \
/tmp/sexnet_nic_rx_packet_gate_and_handoff_v1.log | tail -420
```

## C. Marker evidence
Expected PASS markers:

- `[sexnet.nic.rx.observe.alloc] ... ok=1`
- `[sexnet.nic.rx.observe.desc.link] count=8 separate_bufs=1 ok=1`
- `[sexnet.nic.rx.observe.ring.program] ... ok=1`
- `[sexnet.nic.rx.observe.window.open] max_iters=50000000`
- `[sexnet.nic.rx.observe.poll.done] dd_set>=1 ... ok=1`
- `[sexnet.nic.rx.observe.pkt.parse] len=60 or len>14, ethertype=0x0800 or 0x0806, ok=1`
- `[sexnet.nic.rx.observe.ring.restore] ... rctl_en=1 ok=1`
- `[sexnet.nic.rx.observe.proof.done] dd_set>=1 ok=1`

## D. What was proven
- sexnet-owned temporary RX ring can be programmed.
- host/TAP frame can DMA into sexnet packet buffer.
- descriptor DD can be observed by sexnet.
- minimal Ethernet header parse can be observed.
- original ring/ownership can be restored with `rctl_en=1`.
- no-fault runtime lane is compatible with this temporary proof.

## E. What was not proven
- not permanent NIC ownership transfer.
- not TX ownership or TX data-path proof.
- not ARP/IP/TCP/HTTP state-machine implementation in sexnet.
- not IRQ-driven RX path.
- not non-TAP guarantees.

## F. Architecture boundary
This proof is explicitly temporary observe/restore only.

PCI HAL diagnostic bridge remains preserved. This gate does not redefine ownership boundaries; it only validates the bounded observe window proof when TAP/e1000e + host traffic are present.

## G. STOP FIRST rules
Stop first if any of these occur:

- gate would hard-fail non-TAP boots.
- source observe markers require renaming.
- proof requires kernel/PCI HAL/sex-pdx or other non-allowed file edits.
- restore marker shows failure (`ok=0` or `rctl_en!=1`).

## H. Next missions
1. `SEXNET_NIC_RX_PACKET_GATE_RERUN_V1`
   - verify gate reports PASS under TAP/e1000e.
2. `SEXNET_NIC_RX_OWNERSHIP_TRANSFER_PLAN_V1`
   - plan permanent ownership transfer from HAL diagnostic ring to sexnet.
3. `SEXNET_NIC_TX_STOP_REVIEW_V1`
   - review first user-PD TX proof, no protocol state yet.
4. `SEXNET_ARP_FROM_SEXNET_PLAN_V1`
   - plan ARP after RX/TX ownership is reviewed.
