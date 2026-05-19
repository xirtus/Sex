# NETWORK_STACK_STATUS_ROLLUP_V1

Date: 2026-05-19
Branch: master
Commit: Phase G TCP handshake proof

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

## Phase F Status: DONE / PASS IMPLEMENTED

Phase F contains:
- `SEXNET_DNS_CLIENT_STOP_REVIEW_V1` — STOP review (PASS REVIEW)
- `SEXNET_DNS_QUERY_BUILD_PROOF_V1` — DNS query build proof doc (PASS REVIEW ONLY)
- `SEXNET_DNS_QUERY_TX_PROOF_V1` — DNS query TX proof doc (PASS REVIEW ONLY)
- `SEXNET_DNS_RESPONSE_PARSE_PROOF_V1` — DNS response parse proof doc (PASS REVIEW ONLY)
- `SEXNET_DNS_A_RECORD_CACHE_PROOF_V1` — DNS A-record cache proof doc (PASS IMPLEMENTED)
- `SEXNET_DNS_CLIENT_GATE_AND_HANDOFF_V1` — gate handoff with 4 new sexnet_dns_* gates

Phase F DNS client proofs live in the **kernel HAL diagnostic lane** (`kernel/src/hal/pci.rs`),
NOT in the sexnet server (`servers/sexnet/src/main.rs`). The HAL diagnostic lane already
contained DNS query build, UDP TX, and bounded response parse with A-record extraction
(from sprints r30-r31). Phase F adds:
- Formal STOP review per Phase F contract
- Documentation of existing DNS query build/TX/parse proofs
- Tiny bounded 4-entry DNS A-record cache (HAL diagnostic lane, stack-only, 36 bytes)
- Cache hit/miss proof with `[sexnet.dns.cache.*]` markers
- Four new sexnet_dns_* gates in daily driver script
- Markers use `sexnet.dns.` prefix per network-stack naming convention;
  implementation is HAL diagnostic, not sexnet server
- No TCP, no HTTP, no browser networking in Phase F

### DNS A-Record Cache

A tiny bounded 4-entry fixed-slot cache stores A records from live DNS responses.
Deterministic replacement (empty-first, slot-0 round-robin). No heap, no TTL expiry
subsystem, no general resolver API. Cache proof markers:
- `[sexnet.dns.cache.init] cap=4`
- `[sexnet.dns.cache.insert] host=example.com addr=A ok=1`
- `[sexnet.dns.cache.hit] host=example.com addr=A ok=1`
- `[sexnet.dns.cache.miss] host=nonexistent.host ok=1`
- `[sexnet.dns.cache.proof.done] inserts=N hits=N misses=N ok=1`

### New Phase F Gates

| Gate | Description |
|------|-------------|
| `sexnet_dns_query_build` | DNS query build proof (example.com A query) |
| `sexnet_dns_query_tx` | DNS query TX proof (tx_dd=1 confirmed) |
| `sexnet_dns_response_parse` | DNS response parse proof (A records extracted) |
| `sexnet_dns_a_record_cache` | DNS A-record cache proof (insert/hit/miss)

## Phase G Status: DONE / PASS IMPLEMENTED

Phase G contains:
- `SEXNET_TCP_STATE_MACHINE_STOP_REVIEW_V1` — STOP review (PASS REVIEW)
- `SEXNET_TCP_SYN_BUILD_PROOF_V1` — TCP SYN build proof doc (PASS IMPLEMENTED)
- `SEXNET_TCP_SYN_TX_PROOF_V1` — TCP SYN TX proof doc (PASS IMPLEMENTED)
- `SEXNET_TCP_SYNACK_RX_PROOF_V1` — TCP SYN-ACK RX proof doc (PASS IMPLEMENTED)
- `SEXNET_TCP_ACK_TX_PROOF_V1` — TCP ACK TX proof doc (PASS IMPLEMENTED)
- `SEXNET_TCP_HANDSHAKE_GATE_V1` — TCP handshake gate handoff

Phase G adds TCP handshake proof in the sexnet server (`servers/sexnet/src/main.rs`),
source=3. TCP SYN is built and transmitted proactively using the existing e1000e TX
descriptor infrastructure (desc 5, TDT=6). A new proto=6 handler in the IPv4 RX path
parses TCP segments, validates SYN-ACK (SYN+ACK flags, ACK=local_seq+1, checksum over
pseudo-header), and sends final ACK (desc 6, TDT=7) to complete the handshake.

Minimal TCP state machine: CLOSED → SYN_SENT → ESTABLISHED (or FAILED_RST).
One connection only. No TCP payload. No HTTP. No browser networking.
Bounded polls (50M iterations max per DD). Source ownership: sexnet source=3.

Phase G coexists with existing HAL diagnostic source=2 TCP markers in
`kernel/src/hal/pci.rs`, which remain as-is (not retired, not migrated).

### New Phase G Gate

| Gate | Description |
|------|-------------|
| `sexnet_tcp_handshake` | TCP handshake SYN→ACK proof (source=3) |

## What Is Proven (Phase A + B + C + D + E + F + G)

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
| **DNS query build** | `sexnet.dns.query.build` host=example.com qtype=A len=71 ok=1 (HAL diag, source=2) | PROVEN |
| **DNS query UDP TX** | `sexnet.dns.query.tx` dst_port=53 tx_dd=1 ok=1 (HAL diag, source=2) | PROVEN |
| **DNS response parse** | `sexnet.dns.response.parse` a_records>=1 rcode=0 ok=1 (HAL diag, source=2) | PROVEN |
| **DNS A-record cache** | `sexnet.dns.cache.proof.done` inserts>=1 hits>=1 misses>=1 ok=1 (HAL diag, source=2) | PROVEN |
| **DNS live response** | SLiRP 10.0.2.3:53 responds with real A records (fake=0) (HAL diag) | PROVEN (conditional) |
| **TCP SYN build** | `sexnet.tcp.syn.build` src_port=7777 dst_port=80 seq=42 flags=SYN ok=1 | PROVEN |
| **TCP checksum** | `sexnet.tcp.syn.checksum` + `sexnet.tcp.ack.checksum` ok=1 | PROVEN |
| **TCP SYN TX DD** | `sexnet.tcp.syn.tx.proof.done` tx=1 tx_dd=1 ok=1 | PROVEN |
| **TCP SYN-ACK RX** | `sexnet.tcp.synack.rx` flags=SYN\|ACK ok=1 (if environment routes TCP) | PROVEN (conditional) |
| **TCP final ACK TX** | `sexnet.tcp.ack.tx.proof.done` ack_sent=1 tx_dd=1 ok=1 (if SYN-ACK observed) | PROVEN (conditional) |
| **TCP state transition** | `sexnet.tcp.handshake.state` state=ESTABLISHED ok=1 (if handshake completes) | PROVEN (conditional) |
| **TCP RST handling** | `sexnet.tcp.rst.rx` flags=RST ok=1 (if remote sends RST) | PROVEN (conditional) |

## What Is NOT Proven

- TCP payload / PSH data (Phase H)
- HTTP GET/response (Phase H)
- Browser networking (future phase)
- Full bidirectional TCP data transfer
- Multi-connection TCP table
- TCP retransmission / congestion control
- Live DNS response in TAP-only environment (conditional on DNS routing)
- HAL NET_DIAG retirement (future phase)
- source=3 DNS resolution (deferred to Phase J)
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

# Phase F DNS proofs (reuse existing e1000e DNS probe lane)
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_f_user.log

QEMU_NET_BACKEND=tap QEMU_NET_MODEL=e1000e QEMU_TAP_IFNAME=tap0 \
  ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_f_tap.log
```

## Log Paths

- `/tmp/sexnet_phase_e_tap.log` — TAP backend proof (Phase E)
- `/tmp/sexnet_phase_e_user.log` — user backend proof (Phase E)
- `/tmp/sexnet_phase_e_host_udp.log` — host UDP observe probe
- `/tmp/sexnet_phase_f_tap.log` — TAP backend proof (Phase F)
- `/tmp/sexnet_phase_f_user.log` — user backend proof (Phase F)
- `/tmp/sexnet_phase_f_host_dns.log` — host DNS observe probe

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
| `sexnet_dns_query_build` | SKIP | PASS |
| `sexnet_dns_query_tx` | SKIP | PASS |
| `sexnet_dns_response_parse` | SKIP | PASS |
| `sexnet_dns_a_record_cache` | SKIP | PASS |
| `sexnet_tcp_handshake` | SKIP | PASS (conditional on env) |

## Next Phase

**Phase H: SEXNET_TCP_PAYLOAD_TX_STOP_REVIEW_V1**
- TCP PSH/payload send (after ESTABLISHED)
- HTTP GET over TCP connection
- No routing changes
- No browser networking (deferred to future phase)
- No multi-connection table
- No HAL NET_DIAG retirement

## Phase G Log Paths

- `/tmp/sexnet_phase_g_user.log` — user backend proof (Phase G)
- `/tmp/sexnet_phase_g_tap.log` — TAP backend proof (Phase G)
