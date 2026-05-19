# SEXNET_UDP_ECHO_REPLY_GATE_V1

Date: 2026-05-19
Branch: master
Gate name: `sexnet_udp_echo_reply`

## Old State

Gate did not exist. UDP RX/TX was not implemented in sexnet main loop.
Prior UDP_DNS_PROBE_V1 used an independent out-of-band test path.

## Gate Logic (implemented in scripts/daily_driver_master_gate.sh)

### PASS conditions (all must be true):
1. `sexnet.udp.header.proof.done` with `rx_udp=1 valid=1 ok=1` present
2. `sexnet.udp.echo.proof.done` with `tx_dd=1 ok=1` present (or `tx_reply=1` with DD done)
3. `sexnet.udp.rx.datagram` with `ok=1` present (at least one RX)
4. `sexnet.udp.tx.poll.done` with `dd_set=1` present (at least one TX DD done)
5. No `#PF`, `#GP`, `panic`, `fault.kill`, `KERNEL PANIC` in log

### SKIP conditions:
1. Backend is usernet and no UDP reaches sexnet NIC (no `sexnet.udp.rx.datagram` marker)
2. TAP/tooling unavailable and UDP proof cannot be stimulated
3. Proof profile intentionally disables UDP proof

### FAIL conditions:
1. UDP proof markers absent when TAP/proof mode claims to run
2. `sexnet.udp.rx.datagram` received but no TX reply completion
3. `sexnet.udp.reject` with no successful datagram (all UDP rejected)
4. Fault scan detects fault

## Exact Markers Accepted

| Marker | Required for PASS | Field checks |
|--------|------------------|--------------|
| `sexnet.udp.rx.datagram` | Yes | ok=1 |
| `sexnet.udp.header.validate` | Yes | ok=1 |
| `sexnet.udp.header.proof.done` | Yes | rx_udp=1 valid=1 ok=1 |
| `sexnet.udp.tx.reply.build` | Yes | ok=1 |
| `sexnet.udp.tx.reply.checksum` | Yes | ok=1 |
| `sexnet.ipv4.tx.udp_reply.build` | Yes | checksum=ok ok=1 |
| `sexnet.eth.tx.udp_reply.desc` | Yes | ok=1 |
| `sexnet.udp.tx.poll.done` | Yes | dd_set=1 |
| `sexnet.udp.echo.proof.done` | Yes | ok=1 |
| `sexnet.udp.reject` | Optional (negative path bonus) | ok=1 |

## Proof Command

```bash
# TAP mode (preferred, requires host UDP stimulus)
QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 \
  ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_e_tap.log

# Usernet mode (may SKIP if no UDP stimulus)
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_e_user.log
```

## Log Paths
- `/tmp/sexnet_phase_e_tap.log` — TAP backend proof
- `/tmp/sexnet_phase_e_user.log` — user backend proof

## Fault Count
Target: 0 faults

## Next Phase Task
SEXNET_UDP_HOST_OBSERVE_PROOF_V1
