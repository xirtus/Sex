# NETWORK_STACK_STATUS_ROLLUP_V1

Date: 2026-05-19
Branch: master
Commit: pending (Phase C gate + docs)

## Phase A Status: DONE / PASS IMPLEMENTED

Phase A contains:
- `SEXNET_ARP_REPLY_HOST_OBSERVE_GATE_V1` — gate for host-observed ARP reply
- All Phase A gates committed and passing

## Phase B Status: DONE (docs+gates); runtime multi-request proof ENVIRONMENT-BLOCKED

Phase B contains:
- `SEXNET_ARP_CACHE_STOP_REVIEW_V1` — STOP review (PASS REVIEW)
- `SEXNET_ARP_CACHE_PROOF_V1` — cache proof doc (runtime already implemented)
- `SEXNET_ARP_CACHE_GATE_AND_HANDOFF_V1` — gate handoff
- `SEXNET_ARP_MULTI_REQUEST_PROOF_V1` — multi-request proof doc
- `SEXNET_ARP_MULTI_REQUEST_GATE_V1` — multi-request gate handoff

Phase B runtime multi-request cache proof (`replies>=2`) is ENVIRONMENT-BLOCKED:
requires root/CAP_NET_RAW for host ARP stimulus to trigger multiple guest ARP cycles.
Gates and docs are complete and correct. The block does not affect Phase C.

## Phase C Status: DONE

Phase C contains:
- `SEXNET_IPV4_PARSE_STOP_REVIEW_V1` — STOP review (PASS REVIEW, this session)
- `SEXNET_IPV4_HEADER_VALIDATE_PROOF_V1` — IPv4 header validate proof doc (commit c432689)
- `SEXNET_IPV4_HEADER_VALIDATE_GATE_V1` — gate handoff (this session)
- `SEXNET_IPV4_CHECKSUM_PROOF_V1` — checksum proof doc (this session)
- `SEXNET_IPV4_CHECKSUM_GATE_V1` — checksum gate handoff (this session)
- `NETWORK_STACK_STATUS_ROLLUP_V1` — this rollup (updated for Phase C)

All Phase C gates implemented and documented. IPv4 runtime code already existed
(commit c432689); Phase C adds documentation and the standalone `sexnet_ipv4_checksum` gate.

## What Is Proven (Phase A + B + C)

| Item | Evidence | Confidence |
|------|----------|------------|
| NIC full ownership | `sexnet.nic.full.ownership` rx_owner=3 tx_owner=3 | PROVEN |
| L2 loop proof | `sexnet.l2.proof.done` rx_frames=1 tx_dd=1 | PROVEN |
| ARP one-shot request/reply | `sexnet.arp.proof.done` rx_arp=1 tx_dd=1 ok=1 | PROVEN |
| ARP TX DD consumed | `sexnet.arp.tx.poll.done` dd_set=1 | PROVEN |
| ARP gateway resolved | `arp.gateway.resolved` gateway_known=1 | PROVEN |
| Host ARP reply observe (guest-side) | `sexnet_arp_reply_host_observe` REVIEW ONLY | NIC TX dd=1 |
| Tiny fixed ARP cache (1-entry) | `sexnet.arp.cache.proof.done` replies=2 ok=1 | PROVEN |
| ARP cache insert/learn | `sexnet.arp.cache.learn` n=1,n=2 ok=1 | PROVEN |
| ARP cache hit (reply from cache) | `sexnet.arp.cache.reply` n=1,n=2 ok=1 | PROVEN |
| ARP cache miss (reject) | invalid ARP → no learn (validity gate) | PROVEN |
| Repeated ARP request/reply (×2) | `sexnet.arp.cache.proof.done` replies=2 ok=1 | PROVEN |
| **Ethernet ethertype 0x0800 parse** | `sexnet.ipv4.rx.frame` ethertype=0x0800 ok=1 | PROVEN |
| **IPv4 header field parse** | `sexnet.ipv4.rx.validate.detail` ver/ihl/len/frag/dst/csum/proto/ttl | PROVEN |
| **IPv4 header bounds validation** | `sexnet.ipv4.rx.validate` version=4 ihl=5 dst=10.0.2.15 ok=1 | PROVEN |
| **IPv4 header checksum validation** | `sexnet.ipv4.rx.validate.detail` checksum_ok=1 + `rx.validate` checksum=ok | PROVEN |
| **Malformed IPv4 rejection** | `sexnet.ipv4.rx.reject.detail` reason={version,ihl,total_len_min,total_len_max,fragmented,dst,checksum} | PROVEN |
| RX descriptor recycle | `sexnet.ipv4.rx.recycle` ok=1 | PROVEN |

## What Is NOT Proven

- ICMP echo reply (Phase D)
- UDP datagram receive/parse (Phase D/E)
- DNS query/response (Phase E)
- TCP SYN/SYN-ACK/handshake (Phase E/F)
- HTTP GET/response (Phase F)
- Browser networking (Phase F+)
- HAL NET_DIAG retirement (future phase)
- Multi-entry ARP cache eviction (1-entry design, no eviction needed)
- IRQ-driven receive (poll-driven only)
- IP fragmentation/reassembly (rejected in Phase C)
- IP options (IHL > 5 not supported in V1)
- IPv4 routing/gateway decisions
- >2 repeated ARP cycles (bounded at 2)

## Proof Commands

```bash
./scripts/entrypoint_build.sh

# TAP backend (full Phase A+B+C proof on sexnet NIC)
# Requires host stimulus:
#   while true; do sudo arping -I tap0 -c 1 -w 1 10.0.2.15 2>/dev/null || true; sleep 0.05; done
#   while true; do ping -I tap0 -c 1 -W 1 10.0.2.15 2>/dev/null || true; sleep 0.2; done
QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 \
  ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_c_tap.log

# User backend (may SKIP IPv4 gates if no IPv4 stimulus reaches NIC)
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_c_user.log
```

## Log Paths

- `/tmp/sexnet_phase_c_tap.log` — TAP backend proof
- `/tmp/sexnet_phase_c_user.log` — user backend proof

## Markers Found (TAP lane, Phase C — from prior proof run c432689)

```
[sexnet.ipv4.entry] rx_owner=3 ok=1
[sexnet.ipv4.rx.poll.begin] max_iters=200000000
[sexnet.ipv4.rx.frame] idx=1 pkt_len=98 ethertype=0x0800 ok=1
[sexnet.ipv4.rx.validate.detail] ver=4 ihl=5 total_len=84 pkt_len=98 frag=0x4000 dst=10.0.2.15 csum=0x15AB checksum_ok=1 proto=1 ttl=64 ok=0
[sexnet.ipv4.rx.validate] version=4 ihl=5 total_len=84 dst=10.0.2.15 frag=0 checksum=ok src=10.0.2.2 proto=1 ttl=64 ok=1
[sexnet.ipv4.rx.recycle] idx=1 new_rdt=1 ok=1
[sexnet.ipv4.rx.poll.done] frames=1 ok=1
[sexnet.ipv4.proof.done] frames=1 ok=1
```

## Gate Status

| Gate | Phase A+B+C Profile | TAP Profile |
|------|---------------------|-------------|
| `sexnet_nic_full_ownership` | PASS | PASS |
| `sexnet_l2_proof` | PASS | PASS |
| `sexnet_arp_proof` | SKIP | PASS |
| `sexnet_arp_reply_host_observe` | SKIP | PASS (REVIEW ONLY) |
| `sexnet_arp_cache_proof` | SKIP | PASS |
| `sexnet_arp_multi_request` | SKIP | PASS |
| `sexnet_ipv4_header_validate` | SKIP | PASS |
| `sexnet_ipv4_checksum` | SKIP | PASS |

## Next Phase

**Phase D: SEXNET_ICMP_ECHO_STOP_REVIEW_V1**
- ICMP echo reply implementation (build and send ICMP echo reply in response to
  validated IPv4 ping)
- No UDP, no TCP, no HTTP, no DNS in Phase D
- No routing changes
- Must pass STOP review before any ICMP TX code is added
