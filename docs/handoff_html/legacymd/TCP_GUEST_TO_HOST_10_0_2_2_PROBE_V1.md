# TCP_GUEST_TO_HOST_10_0_2_2_PROBE_V1

Date: 2026-05-17
Model: QEMU_NET_MODEL=e1000e
Backend: QEMU user-net (SLiRP)

## Goal
Probe guest->host TCP path via SLiRP gateway (`10.0.2.2`) with a controlled local host listener and bounded SYN retries.

Constraints preserved:
- no final ACK unless SYN-ACK seen
- no HTTP GET
- bounded retries and polls only

## Runtime

```bash
python3 -m http.server 18080 --bind 0.0.0.0
QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 QEMU_NET_BACKEND=user \
  ./scripts/run_daily_driver_proof.sh /tmp/sexos_tcp_guest_host_10_0_2_2_probe_v1.log
```

Gate result: **FINAL PASS (239 gates, 0 fail, 13 skip)**.

## Probe Evidence

- `[tcp.guest.host.10_0_2_2.plan] dst_ip=10.0.2.2 dst_port=18080 ...`
- attempts (source-port rotation):
  - attempt1: `src_port=49153` -> `10.0.2.2:18080`, `tx_dd=1`
  - attempt2: `src_port=49154` -> `10.0.2.2:18080`, `tx_dd=1`
  - attempt3: `src_port=49155` -> `10.0.2.2:18080`, `tx_dd=1`
- `[tcp.syn.rx.synack] ... synack_seen=0 rst_seen=0 ...`
- `[tcp.guest.host.10_0_2_2.probe.done] ... synack_seen=0 rst_seen=0 final_ack_sent=0 http_sent=0 ok=1 ...`

## Deferral Truth

- `[tcp.handshake.ack.tx.post] ... sent=0 ...` (final ACK deferred)
- `[http.get.send.proof] sent=0 ...` (HTTP deferred)

## Backend Truth

- `[qemu.net.config] backend=user model=e1000e usernet=1 hostfwd=none tap_if=tap0`

## Interpretation

Even against SLiRP gateway host path and a local host listener target, SYN TX is real (`tx_dd=1`) but no SYN-ACK and no RST are observed in bounded windows.
This strengthens the diagnosis that remaining blocker is backend/policy behavior for this raw TCP path rather than ARP/L2 or target-domain choice.
