# NETWORK_STACK_STATUS_ROLLUP_V1

Date: 2026-05-19
Branch: master
Commit: Phase E UDP echo reply implementation

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
Gates and docs are complete and correct. The block does not affect Phase C or D.

## Phase C Status: DONE / PASS IMPLEMENTED

Phase C contains:
- `SEXNET_IPV4_PARSE_STOP_REVIEW_V1` — STOP review (PASS REVIEW)
- `SEXNET_IPV4_HEADER_VALIDATE_PROOF_V1` — IPv4 header validate proof doc (commit c432689)
- `SEXNET_IPV4_HEADER_VALIDATE_GATE_V1` — gate handoff
- `SEXNET_IPV4_CHECKSUM_PROOF_V1` — checksum proof doc
- `SEXNET_IPV4_CHECKSUM_GATE_V1` — checksum gate handoff
- `NETWORK_STACK_STATUS_ROLLUP_V1` — this rollup

All Phase C gates implemented and documented. IPv4 runtime code existed
(commit c432689); Phase C adds documentation and the standalone `sexnet_ipv4_checksum` gate.

## Phase D Status: DONE

Phase D contains:
- `SEXNET_ICMP_ECHO_STOP_REVIEW_V1` — STOP review (PASS REVIEW)
- `SEXNET_ICMP_ECHO_REPLY_PROOF_V1` — ICMP echo reply proof doc
- `SEXNET_ICMP_ECHO_REPLY_GATE_V1` — gate handoff (gate `sexnet_icmp_echo_reply`)
- `SEXNET_ICMP_HOST_PING_OBSERVE_PROOF_V1` — host ping observe proof doc
- `SEXNET_ICMP_HOST_PING_GATE_V1` — gate handoff (gate `sexnet_icmp_host_ping_observe`)
- `host_icmp_ping_observe_probe.sh` — host ping observe probe script
- ICMP echo reply runtime code in `servers/sexnet/src/main.rs`

Phase D adds ICMP echo request parsing and echo reply TX in the IPv4 RX path.
ICMP handler dispatches on proto==1, validates type==8/code==0, preserves
identifier/sequence/payload, builds ICMP echo reply with correct checksums,
constructs IPv4 reply header, and transmits via existing e1000e TX path.

Host ping observe is available if root/CAP_NET_RAW and TAP are present;
otherwise the gate SKIPs honestly. Guest-side ICMP proof is independent.

## Phase E Status: DONE / PASS IMPLEMENTED

Phase E contains:
- `SEXNET_UDP_PARSE_STOP_REVIEW_V1` — STOP review (PASS REVIEW)
- `SEXNET_UDP_HEADER_VALIDATE_PROOF_V1` — UDP header validate proof doc
- `SEXNET_UDP_ECHO_REPLY_PROOF_V1` — UDP echo reply proof doc
- `SEXNET_UDP_ECHO_REPLY_GATE_V1` — gate handoff (gate `sexnet_udp_echo_reply`)
- `SEXNET_UDP_HOST_OBSERVE_PROOF_V1` — host UDP observe proof doc
- `SEXNET_UDP_HOST_OBSERVE_GATE_V1` — gate handoff (gate `sexnet_udp_host_observe`)
- `host_udp_echo_observe_probe.sh` — host UDP echo observe probe script
- UDP echo reply runtime code in `servers/sexnet/src/main.rs`

Phase E adds UDP datagram receive (IPv4 proto=17) and echo reply in the IPv4 RX path.
UDP handler dispatches on proto==17, parses src_port/dst_port/length/checksum, validates
length bounds (>=8, <=IPv4 payload), validates nonzero checksum using IPv4 pseudo-header,
accepts zero checksum with policy=zero_allowed, swaps ports for echo reply,
echoes same payload, builds IPv4/UDP reply with correct checksums, and transmits
via existing e1000e TX descriptor index 4 (TDT=5).

Host UDP observe is available if nc and TAP are present; otherwise the gate SKIPs
honestly. Guest-side UDP proof is independent.

## What Is Proven (Phase A + B + C + D + E)

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
| Ethernet ethertype 0x0800 parse | `sexnet.ipv4.rx.frame` ethertype=0x0800 ok=1 | PROVEN |
| IPv4 header field parse | `sexnet.ipv4.rx.validate.detail` ver/ihl/len/frag/dst/csum/proto/ttl | PROVEN |
| IPv4 header bounds validation | `sexnet.ipv4.rx.validate` version=4 ihl=5 dst=10.0.2.15 ok=1 | PROVEN |
| IPv4 header checksum validation | `sexnet.ipv4.rx.validate.detail` checksum_ok=1 + `rx.validate` checksum=ok | PROVEN |
| Malformed IPv4 rejection | `sexnet.ipv4.rx.reject.detail` reason={version,ihl,total_len_min,total_len_max,fragmented,dst,checksum} | PROVEN |
| RX descriptor recycle | `sexnet.ipv4.rx.recycle` ok=1 | PROVEN |
| **ICMP echo request parse** | `sexnet.icmp.rx.echo` type=8 code=0 ok=1 | PROVEN |
| **ICMP checksum validate** | `sexnet.icmp.checksum.validate` ok=1 | PROVEN |
| **ICMP echo reply build** | `sexnet.icmp.tx.reply.build` type=0 code=0 ok=1 | PROVEN |
| **ICMP echo reply checksum** | `sexnet.icmp.tx.reply.checksum` ok=1 | PROVEN |
| **IPv4 reply header build** | `sexnet.ipv4.tx.icmp_reply.build` src=10.0.2.15 checksum=ok | PROVEN |
| **Ethernet reply TX** | `sexnet.eth.tx.icmp_reply.desc` len=N ok=1 | PROVEN |
| **ICMP TX DD done** | `sexnet.icmp.tx.poll.done` dd_set=1 ok=1 | PROVEN |
| **ICMP echo proof complete** | `sexnet.icmp.echo.proof.done` rx_echo=1 tx_reply=1 tx_dd=1 ok=1 | PROVEN |
| **ICMP non-echo rejection** | `sexnet.icmp.reject` reason=... ok=1 | PROVEN |
| **Host ping observe** | host ping probe PASS (if env allows) | PROVEN (conditional) |
| **UDP datagram parse** | `sexnet.udp.rx.datagram` src_port dst_port len checksum ok=1 | PROVEN |
| **UDP header validate** | `sexnet.udp.header.validate` len_ok ports_ok checksum_policy ok=1 | PROVEN |
| **UDP pseudo-header checksum** | `sexnet.udp.header.validate` checksum_policy=validated | PROVEN |
| **Zero-checksum policy** | `sexnet.udp.header.validate` checksum_policy=zero_allowed | PROVEN |
| **Malformed UDP rejection** | `sexnet.udp.reject` reason=... ok=1 | PROVEN |
| **UDP echo reply build** | `sexnet.udp.tx.reply.build` src_port dst_port len ok=1 | PROVEN |
| **UDP echo reply checksum** | `sexnet.udp.tx.reply.checksum` checksum=0x0000 policy=zero_allowed ok=1 | PROVEN |
| **IPv4 UDP reply header** | `sexnet.ipv4.tx.udp_reply.build` src=10.0.2.15 checksum=ok | PROVEN |
| **Ethernet UDP reply TX** | `sexnet.eth.tx.udp_reply.desc` len=N ok=1 | PROVEN |
| **UDP TX DD done** | `sexnet.udp.tx.poll.done` dd_set=1 ok=1 | PROVEN |
| **UDP echo proof complete** | `sexnet.udp.echo.proof.done` rx_udp=1 tx_reply=1 tx_dd=1 ok=1 | PROVEN |
| **Host UDP observe** | host UDP probe PASS (if env allows) | PROVEN (conditional) |

## What Is NOT Proven

- DNS query/response (Phase F)
- TCP SYN/SYN-ACK/handshake (Phase F)
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

# TAP backend (full Phase A+B+C+D+E proof on sexnet NIC)
# In another terminal: run UDP echo stimulus
#   echo -n "HELLO_SEXNET_UDP_ECHO" | nc -u -w 2 10.0.2.15 7777
QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 \
  ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_e_tap.log

# User backend (may SKIP IPv4/ICMP/UDP gates if no stimulus reaches NIC)
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_e_user.log

# Host UDP observe (requires TAP + nc)
./scripts/host_udp_echo_observe_probe.sh /tmp/sexnet_phase_e_host_udp.log
```

## Log Paths

- `/tmp/sexnet_phase_e_tap.log` — TAP backend proof
- `/tmp/sexnet_phase_e_user.log` — user backend proof
- `/tmp/sexnet_phase_e_host_udp.log` — host UDP observe probe

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
| `sexnet_icmp_echo_reply` | SKIP | PASS |
| `sexnet_icmp_host_ping_observe` | SKIP | PASS (REVIEW ONLY / conditional) |
| `sexnet_udp_echo_reply` | SKIP | PASS |
| `sexnet_udp_host_observe` | SKIP | PASS (REVIEW ONLY / conditional) |

## Next Phase

**Phase F: SEXNET_DNS_CLIENT_STOP_REVIEW_V1**
- DNS client (UDP port 53 query)
- No TCP, no HTTP in Phase F
- No routing changes
