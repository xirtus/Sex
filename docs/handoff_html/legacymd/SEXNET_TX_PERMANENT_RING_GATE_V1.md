# SEXNET_TX_PERMANENT_RING_GATE_V1

## A. Result
Added three daily-driver gates for permanent TX ownership and SEXNET_FULL ownership:

- `sexnet_nic_tx_permanent_init`
- `sexnet_nic_tx_permanent_send`
- `sexnet_nic_full_ownership`

All gates remain `SKIP` on non-enabled/non-NIC boots.

## B. Proof command / preconditions
```bash
QEMU_NET_BACKEND=tap \
QEMU_NET_MODEL=e1000e \
QEMU_TAP_IFNAME=tap0 \
ENABLE_QEMU_USERNET_E1000=1 \
./scripts/run_daily_driver_proof.sh /tmp/sexnet_tx_permanent_ring_gate_v1.log
```

Scan:

```bash
grep -E "sexnet_nic_tx_permanent|sexnet_nic_full_ownership|sexnet.nic.tx.permanent|fault.kill|#PF|#GP|panic|KERNEL PANIC|FINAL:" \
/tmp/sexnet_tx_permanent_ring_gate_v1.log | tail -420
```

## C. Marker evidence
Gate `sexnet_nic_tx_permanent_init` PASS requires:

- `[sexnet.nic.tx.permanent.claim] owner=2 ring_ok=1 ok=1`

Gate `sexnet_nic_tx_permanent_send` PASS requires:

- `[sexnet.nic.tx.permanent.poll.done] dd_set=1 desc_idx=0 ok=1`

Gate `sexnet_nic_full_ownership` PASS requires:

- `[sexnet.nic.tx.permanent.full] rx_owner=3 tx_owner=3 full_ok=1`

## D. What was proven
- Permanent raw TX claim contract can be proven from marker evidence.
- TX descriptor consumption (DD=1) can be proven in permanent TX lane.
- Combined full ownership state (`SEXNET_FULL`) can be proven from marker contract.

## E. What was not proven
- Not ARP/IP/TCP/HTTP protocol ownership yet.
- Not IRQ-driven network pipeline.
- Not browser fetch via sexnet.

## F. NET_DIAG/browser impact
NET_DIAG/browser real HTTP-body evidence still comes from HAL boot diagnostic atomics; these gates do not change that source contract.

## G. Architecture boundary
This proves permanent raw RX/TX NIC ownership inside sexnet marker contracts. PCI HAL diagnostic bridge remains present/fallback.

## H. STOP FIRST rules
Stop first if any of these occur:

- Gates start requiring protocol markers.
- Gates require browser/HAL behavior changes.
- Gates hard-fail non-enabled boots.
- Marker names drift and require source-code edits.
- Any non-allowed code file is required.

## I. Next missions
1. `SEXNET_L2_RX_TX_LOOP_PLAN_V1`
2. `SEXNET_ARP_STATE_MACHINE_STOP_REVIEW_V1`
3. `SEXNET_NETDIAG_SOURCE3_PLAN_V1`
4. `HAL_NET_DIAG_DEPRECATION_PLAN_V1`
