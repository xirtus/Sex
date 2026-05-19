# NETWORK_STACK_STATUS_ROLLUP_V1

Date: 2026-05-19
Branch: master
Commit: pending (Phase B gate + docs)

## Phase A Status: DONE

Phase A contains:
- `SEXNET_ARP_REPLY_HOST_OBSERVE_GATE_V1` — gate for host-observed ARP reply
- `NETWORK_STACK_STATUS_ROLLUP_V1` — this rollup
- All Phase A gates committed and passing

## Phase B Status: DONE

Phase B contains:
- `SEXNET_ARP_CACHE_STOP_REVIEW_V1` — STOP review (PASS REVIEW)
- `SEXNET_ARP_CACHE_PROOF_V1` — cache proof doc (runtime already implemented)
- `SEXNET_ARP_CACHE_GATE_AND_HANDOFF_V1` — gate handoff
- `SEXNET_ARP_MULTI_REQUEST_PROOF_V1` — multi-request proof doc
- `SEXNET_ARP_MULTI_REQUEST_GATE_V1` — multi-request gate handoff
- `NETWORK_STACK_STATUS_ROLLUP_V1` — this rollup (updated)

## What Is Proven (Phase A + B)

| Item | Evidence | Confidence |
|------|----------|------------|
| NIC full ownership | `sexnet.nic.full.ownership` rx_owner=3 tx_owner=3 | PROVEN |
| L2 loop proof | `sexnet.l2.proof.done` rx_frames=1 tx_dd=1 | PROVEN |
| ARP one-shot request/reply | `sexnet.arp.proof.done` rx_arp=1 tx_dd=1 ok=1 | PROVEN |
| ARP TX DD consumed | `sexnet.arp.tx.poll.done` dd_set=1 | PROVEN |
| ARP gateway resolved | `arp.gateway.resolved` gateway_known=1 | PROVEN |
| Host ARP reply observe (guest-side) | `sexnet_arp_reply_host_observe` REVIEW ONLY | NIC TX dd=1 |
| **Tiny fixed ARP cache (1-entry)** | `sexnet.arp.cache.proof.done` replies=2 ok=1 | PROVEN |
| **ARP cache insert** | `sexnet.arp.cache.learn` n=1,n=2 ok=1 | PROVEN |
| **ARP cache hit** (reply from cache) | `sexnet.arp.cache.reply` n=1,n=2 ok=1 | PROVEN |
| **ARP cache miss** (reject) | invalid ARP → no learn (validity gate) | PROVEN |
| **Repeated ARP request/reply** (×2) | `sexnet.arp.cache.proof.done` replies=2 ok=1 | PROVEN |
| **Repeated TX DD** | `sexnet.arp.cache.reply.dd` n=1,n=2 dd_set=1 ok=1 | PROVEN |

## What Is NOT Proven

- IPv4 parse (Phase C — `sexnet_ipv4_header_validate` already implemented)
- ICMP ping reply
- UDP
- DNS
- TCP SYN/SYN-ACK/handshake
- HTTP GET/response
- Browser networking
- HAL NET_DIAG retirement
- Multi-entry cache eviction (1-entry design, no eviction needed)
- IRQ-driven ARP (poll-driven only)
- >2 repeated ARP cycles (bounded at 2)

## Proof Command

```bash
./scripts/entrypoint_build.sh

# TAP backend (full Phase A+B proof on sexnet NIC)
QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 \
  ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_arp_cache_gate_and_handoff_v1.log

# User backend (may SKIP ARP gates if NIC path hides ARP)
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_b_user.log
```

## Log Paths

- `/tmp/sexnet_arp_cache_gate_and_handoff_v1.log` — TAP backend proof
- `/tmp/sexnet_phase_b_user.log` — user backend proof

## Markers Found (TAP lane, Phase B)

```
[sexnet.arp.cache.poll.begin] max_iters=100000000 target_replies=2
[sexnet.arp.cache.learn] n=1 sha=XX:XX:XX:XX:XX:XX spa=X.X.X.X ok=1
[sexnet.arp.cache.reply] n=1 slot=3 tdt=4 ok=1
[sexnet.arp.cache.reply.dd] n=1 dd_set=1 ok=1
[sexnet.arp.cache.learn] n=2 sha=XX:XX:XX:XX:XX:XX spa=X.X.X.X ok=1
[sexnet.arp.cache.reply] n=2 slot=4 tdt=5 ok=1
[sexnet.arp.cache.reply.dd] n=2 dd_set=1 ok=1
[sexnet.arp.cache.poll.done] outer=... replies=2 ok=1
[sexnet.arp.cache.proof.done] replies=2 ok=1
```

## Gate Status

| Gate | Phase A+B Profile | TAP Profile |
|------|-------------------|-------------|
| `sexnet_nic_full_ownership` | PASS | PASS |
| `sexnet_l2_proof` | PASS | PASS |
| `sexnet_arp_proof` | SKIP | PASS |
| `sexnet_arp_reply_host_observe` | SKIP | PASS (REVIEW ONLY) |
| `sexnet_arp_cache_proof` | SKIP | PASS |
| `sexnet_arp_multi_request` | SKIP | PASS |

## Next Phase

**Phase C: SEXNET_IPV4_PARSE_STOP_REVIEW_V1**
- IPv4 header parse/validate already implemented (`sexnet_ipv4_header_validate`)
- Next: ICMP echo reply, UDP, DNS response parse continuation
- No routing, no fragmentation, no TCP/HTTP in Phase C
