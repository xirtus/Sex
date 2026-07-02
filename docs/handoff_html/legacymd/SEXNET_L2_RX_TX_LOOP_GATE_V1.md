# SEXNET_L2_RX_TX_LOOP_GATE_V1

## A. Result
Added three daily-driver gates for bounded sexnet Layer-2 RX/TX loop proof:

- `sexnet_l2_rx_loop`
- `sexnet_l2_tx_reuse`
- `sexnet_l2_proof`

These gates are traffic-aware and default to `SKIP` on absent/non-enabled lanes.

## B. Proof command / host preconditions
Run ARP traffic first:

```bash
while true; do
  sudo arping -I tap0 -c 1 -b 10.0.2.15 2>/dev/null || true
  sleep 0.05
done
```

Run proof:

```bash
QEMU_NET_BACKEND=tap \
QEMU_NET_MODEL=e1000e \
QEMU_TAP_IFNAME=tap0 \
ENABLE_QEMU_USERNET_E1000=1 \
./scripts/run_daily_driver_proof.sh /tmp/sexnet_l2_rx_tx_loop_gate_v1.log
```

Scan:

```bash
grep -E "sexnet_l2_|sexnet.l2|fault.kill|#PF|#GP|panic|KERNEL PANIC|FINAL:" \
/tmp/sexnet_l2_rx_tx_loop_gate_v1.log | tail -420
```

## C. Marker evidence
`sexnet_l2_rx_loop` PASS requires:

- `[sexnet.l2.entry] rx_owner=3 tx_owner=3 ok=1`
- `[sexnet.l2.rx.poll.done] frames_rx>=1 ok=1`
- Either:
  - `[sexnet.l2.rx.recycle] ... ok=1`
  - OR ARP-preserve path:
    - `[sexnet.l2.rx.frame] ... ethertype=0x0806 ... ok=1`
    - `[sexnet.arp.proof.done] rx_arp=1 tx_dd=1 ok=1`

`sexnet_l2_tx_reuse` PASS requires:

- `[sexnet.l2.tx.reuse.desc] slot=2 len=60 ok=1`
- `[sexnet.l2.tx.reuse.post] tdt=3 ok=1`
- `[sexnet.l2.tx.reuse.poll.done] dd_set=1 desc_idx=2 ok=1`

`sexnet_l2_proof` PASS requires:

- `[sexnet.l2.proof.done] rx_frames>=1 tx_dd=1 ok=1`

## D. What was proven
- Bounded L2 RX loop can observe and recycle descriptors with `RDT=idx`.
- Permanent TX ring descriptor slot 1 can be reused and consumed.
- Combined bounded L2 proof contract can complete without faults.

## E. What was not proven
- No ARP state machine.
- No IP/TCP/HTTP/DNS protocol behavior.
- No IRQ-driven networking path.
- No browser path replacement.

## F. Architecture boundary
This proves bounded Layer-2 RX/TX mechanics only and preserves the existing HAL diagnostic fallback contract.

## G. STOP FIRST rules
Stop first if any of these occur:

- Gate starts requiring ARP/IP/TCP/HTTP markers.
- Gate requires browser/HAL behavior changes.
- Gate hard-fails non-TAP/non-enabled boots.
- Marker names drift and require source edits.
- Any non-allowed code file is required.

## H. Next missions
1. `SEXNET_ARP_STATE_MACHINE_STOP_REVIEW_V1`
2. `SEXNET_L2_MULTI_FRAME_REUSE_PROOF_V1`
3. `SEXNET_NETDIAG_SOURCE3_PLAN_V1`
4. `HAL_NET_DIAG_DEPRECATION_PLAN_V1`
