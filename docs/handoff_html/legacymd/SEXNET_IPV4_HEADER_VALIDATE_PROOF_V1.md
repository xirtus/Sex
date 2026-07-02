# SEXNET_IPV4_HEADER_VALIDATE_PROOF_V1

Date: 2026-05-19
Commit: c432689 sexnet: prove IPv4 header validation
Gate: `sexnet_ipv4_header_validate`

## A. Result

IPv4 header receive, parse, and validation proven on e1000e TAP lane. One real IPv4 frame (ICMP echo request from host ping) received, parsed, and validated.

## B. Proof Command / Host Preconditions

### Start ARP flood:
```
while true; do
  sudo arping -I tap0 -c 1 -w 1 10.0.2.15 2>/dev/null || true
  sleep 0.05
done
```

### Start ping flood (separate terminal):
```
while true; do
  ping -I tap0 -c 1 -W 1 10.0.2.15 2>/dev/null || true
  sleep 0.2
done
```

### Run proof:
```
QEMU_NET_BACKEND=tap \
QEMU_NET_MODEL=e1000e \
QEMU_TAP_IFNAME=tap0 \
ENABLE_QEMU_USERNET_E1000=1 \
./scripts/run_daily_driver_proof.sh /tmp/sexnet_ipv4_header_validate_gate_v1.log
```

### Scan:
```
grep -E "sexnet_ipv4|sexnet.ipv4|sexnet.arp.cache|fault.kill|#PF|#GP|panic|KERNEL PANIC|FINAL:" \
/tmp/sexnet_ipv4_header_validate_gate_v1.log | tail -520
```

## C. Marker Evidence

Required markers for PASS:

| Marker | Required Fields |
|--------|-----------------|
| `[sexnet.ipv4.entry]` | `rx_owner=3`, `ok=1` |
| `[sexnet.ipv4.rx.frame]` | `ethertype=0x0800`, `ok=1` |
| `[sexnet.ipv4.rx.validate]` | `version=4`, `ihl=5`, `dst=10.0.2.15`, `checksum=ok`, `ok=1` |
| `[sexnet.ipv4.rx.recycle]` | `ok=1` |
| `[sexnet.ipv4.proof.done]` | `frames=1`, `ok=1` |

Observed evidence from proof run:

```
[sexnet.arp.cache.proof.done] replies=2 ok=1
[sexnet.ipv4.entry] rx_owner=3 ok=1
[sexnet.ipv4.rx.poll.begin] max_iters=200000000
[sexnet.ipv4.rx.frame] idx=1 pkt_len=98 ethertype=0x0800 ok=1
[sexnet.ipv4.rx.validate.detail] ver=4 ihl=5 total_len=84 pkt_len=98 frag=0x4000 dst=10.0.2.15 csum=0x15AB checksum_ok=1 proto=1 ttl=64 ok=0
[sexnet.ipv4.rx.validate] version=4 ihl=5 total_len=84 dst=10.0.2.15 frag=0 checksum=ok src=10.0.2.2 proto=1 ttl=64 ok=1
[sexnet.ipv4.rx.recycle] idx=1 new_rdt=1 ok=1
[sexnet.ipv4.rx.poll.done] frames=1 ok=1
[sexnet.ipv4.proof.done] frames=1 ok=1
```

## D. What Was Proven

- IPv4 frame reception from host ping stimulus on TAP/e1000e lane.
- IPv4 header field parse: version (4), IHL (5), total_length, protocol (1=ICMP), TTL, source/destination IP, fragmentation flags, header checksum.
- Header checksum validation passes (`checksum_ok=1`).
- Frame is recognized as IPv4 (`ethertype=0x0800`).
- RX descriptor recycle discipline (`new_rdt=1`).
- Poll-driven receive loop completes one frame scan.

## E. What Was Not Proven

- **No ICMP echo reply** — this proof only validates IPv4 receive; no ICMP response is sent.
- **No UDP** — no UDP datagrams received or parsed.
- **No TCP** — no TCP segments received or parsed.
- **No HTTP** — no HTTP payloads received.
- **No DNS** — no DNS messages received.
- **No routing** — no IPv4 routing decisions made.
- **No fragmentation/reassembly** — single unfragmented frame (DF=1 in `frag=0x4000`).
- **No IP options** — header is standard 20-byte (IHL=5).
- **No IRQ-driven receive** — poll-driven only.
- **Browser/NET_DIAG** — still not replaced by this proof.

## F. Architecture Boundary

- Proof lives in `servers/sexnet/` (sexnet server).
- Bound to `QEMU_NET_MODEL=e1000e` + `QEMU_NET_BACKEND=tap` lane.
- SKIP on non-TAP or default e1000 boots (no gate failures).
- No source code changes were made for this gate+handoff.

## G. STOP FIRST Rules

- If `[sexnet.ipv4.proof.done]` exists with `ok=0`: FAIL.
- If `[sexnet.ipv4.rx.validate]` exists with `ok=0` and no later `ok=1`: FAIL.
- If `[sexnet.ipv4.entry]` exists with `ok=0`: FAIL.
- If fault/panic markers appear (`fault.kill`, `#PF`, `#GP`, `panic`, `KERNEL PANIC`): FAIL.

## H. Next Missions

1. `SEXNET_ICMP_ECHO_STOP_REVIEW_V1` — gate the ICMP echo reply path (build+send ICMP echo reply in response to validated IPv4 ping).

2. `SEXNET_UDP_RX_OBSERVE_V1` — extend IPv4 receive to UDP protocol parsing.

3. `SEXNET_TCP_RX_OBSERVE_V1` — extend IPv4 receive to TCP protocol parsing.
