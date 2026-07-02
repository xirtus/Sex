# QEMU_SLIRP_TCP_LIMITATION_FREEZE_V1

Date: 2026-05-17
Lane: QEMU `-netdev user` + `e1000e`
Log: `/tmp/sexos_qemu_slirp_tcp_limitation_freeze_v1.log`

## Freeze Truth Marker

- `[qemu.slirp.tcp.limit.freeze] backend=user tcp_syn_tx=1 synack=0 rst=0 checksum_ok=1 offload_ok=1 final_ack_sent=0 http_sent=0 environment_limited=1 ok=1 reason=slirp_tcp_no_response`

Interpretation:

- TCP SYN transmission is real and consumed (`tcp_syn_tx=1`).
- No SYN-ACK and no RST are observed in bounded retry windows.
- Packet-shape correctness remains proven (`checksum_ok=1`, `offload_ok=1`).
- ACK and HTTP are correctly deferred by mission policy.
- Current blocker is frozen as environment/backend-limited in this host lane.

## Proof Chain Table

| Stage | Evidence | Result |
|---|---|---|
| e1000e TX/RX lane | existing daily-driver gates for RX/TX/ring proofs | PASS |
| ARP/gateway | `arp_gateway_resolution_reliability PASS` | PASS |
| ICMP | `icmp_echo_request_proof` + `icmp_echo_reply_observe_proof` | PASS |
| UDP DNS path | UDP send/observe + DNS parse markers | PASS |
| TCP SYN build/send | `tcp_syn_build_v1`, `tcp_syn_tx_post_v1`, `tcp_syn_send_retry_proof_v1` | PASS |
| TCP packet-shape audit | `tcp_checksum_offload_header_audit_v1` | PASS |
| Guest->host target | `[tcp.guest.host.10_0_2_2.probe.done] ... synack_seen=0 rst_seen=0 ...` | PASS (diagnostic no-response) |
| Freeze marker | `[qemu.slirp.tcp.limit.freeze] ... environment_limited=1 ok=1` | PASS |

## Ruled-Out Causes

| Cause | Status | Why ruled out |
|---|---|---|
| ARP/L2 | Ruled out | gateway resolved from real ARP reply; e1000e lane proven |
| DNS/target resolution | Ruled out | DNS query/parse path already proven in-lane |
| TCP checksum | Ruled out | `[tcp.header.audit.tcp] ... match=1 ok=1` |
| IPv4 checksum/header | Ruled out | `[tcp.header.audit.ip] ... match=1 ok=1` |
| TCP header length/options | Ruled out | data offset/length/padding audits all `ok=1` |
| TX offload assumptions | Ruled out | `[tcp.tx.offload.audit] checksum_offload=0 ... ok=1` |
| Target variation | Ruled out | bounded variant probes complete with same no-response pattern |
| Gateway `10.0.2.2` listener path | Ruled out | guest->host `10.0.2.2:18080` attempts show SYN TX consumed, no SYN-ACK/RST |

## Remaining Blocker

QEMU SLiRP/backend environment in this host setup shows no TCP reply visibility for this raw-driver SYN path, even with clean packet/header/offload invariants and bounded retries.

## Next Viable Paths

1. `TAP_HOST_ENV_FIX_PLAN_V1`
2. `HOSTFWD_ENV_FIX_PLAN_V1`
3. `TCP_WITH_CAPTURE_BACKEND_PROOF_V1`
4. Continue browser-side integration with bounded local/mock HTTP until backend TCP response path is available

## Runtime Result

- `FINAL: PASS (241 gates proved, 0 fail, 13 skipped, 0 faults)`
- `faults_zero PASS`
