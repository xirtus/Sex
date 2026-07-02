# SEXNET_RX_PERMANENT_RING_GATE_V1

## A. Result
Added two daily-driver gates for permanent sexnet RX ownership proof:

- `sexnet_nic_rx_permanent_init`
- `sexnet_nic_rx_permanent_recv`

Both gates are TAP/traffic-aware and keep ordinary non-enabled boots non-failing (`SKIP`).

## B. Proof command / host preconditions
Start host ARP traffic before QEMU:

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
./scripts/run_daily_driver_proof.sh /tmp/sexnet_rx_permanent_ring_gate_v1.log
```

Scan:

```bash
grep -E "sexnet_nic_rx_permanent|sexnet.nic.rx.permanent|fault.kill|#PF|#GP|panic|KERNEL PANIC|FINAL:" \
/tmp/sexnet_rx_permanent_ring_gate_v1.log | tail -420
```

## C. Marker evidence
Gate `sexnet_nic_rx_permanent_init` PASS requires:

- `[sexnet.nic.rx.permanent.claim] owner=1 ring_ok=1 ok=1`

Gate `sexnet_nic_rx_permanent_recv` PASS requires:

- `[sexnet.nic.rx.permanent.poll.done] dd_set=1 ... ok=1`
- `[sexnet.nic.rx.permanent.pkt.parse] len>14 ethertype=0x0806 or 0x0800 ok=1`
- `[sexnet.nic.rx.permanent.rdt.advance] ... ok=1`

## D. What was proven
- Permanent RX claim reached owner contract (`owner=1 ring_ok=1`) when init gate passes.
- Receive lane can prove DMA arrival + parse + descriptor recycle when recv gate passes.

## E. What was not proven
- TX is not permanent ownership yet.
- No ARP/IP/TCP/HTTP state machine exists in sexnet.
- No kernel/HAL ownership flag contract exists yet.

## F. NET_DIAG/browser impact
NET_DIAG/browser proof remains sourced from HAL boot diagnostic atomics; this RX permanent gate does not change that contract.

## G. Architecture boundary
This proves permanent RX ownership only. TX remains temporary observe/restore. HAL diagnostic bridge remains preserved.

## H. STOP FIRST rules
Stop first if any of these occur:

- Gate starts requiring TX permanent proof.
- Gate requires kernel/HAL changes.
- Gate hard-fails non-TAP/non-enabled boots by default.
- Marker names drift and require source-code edits.
- Any non-allowed code file is required.

## I. Next missions
1. `SEXNET_TX_PERMANENT_RING_STOP_REVIEW_V1`
