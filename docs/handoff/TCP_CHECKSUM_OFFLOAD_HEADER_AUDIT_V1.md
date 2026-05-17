# TCP_CHECKSUM_OFFLOAD_HEADER_AUDIT_V1

Date: 2026-05-17
Lane: QEMU usernet + `e1000e`
Log: `/tmp/sexos_tcp_checksum_offload_header_audit_v1.log`

## Scope

Audit and prove TCP SYN packet IP/TCP header, checksum, length/padding, and TX offload invariants seen by SLiRP.

Guardrails preserved:

- No final ACK
- No HTTP GET
- No fake SYN-ACK / no fake gateway
- Bounded SYN retries only
- No RDH write introduced

## Runtime

```bash
./scripts/entrypoint_build.sh
QEMU_NET_BACKEND=user QEMU_NET_MODEL=e1000e ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexos_tcp_checksum_offload_header_audit_v1.log
```

Result: **FINAL PASS (240 gates, 0 fail, 13 skip, 0 faults)**

## Byte Layout Table (audited SYN)

| Layer | Field | Value |
|---|---|---|
| Ethernet | dst/src/ethertype | `gw_mac` / `52:54:00:12:34:56` / `0x0800` |
| IPv4 | version+ihl | `0x45` (ihl=20) |
| IPv4 | total_len | `44` |
| IPv4 | ttl/proto | `64` / `6` |
| IPv4 | src/dst | `10.0.2.15` -> `10.0.2.2` |
| TCP | src_port/dst_port | `49153` -> `18080` |
| TCP | seq/ack | `0` / `0` |
| TCP | data_offset | `24` bytes (MSS option present) |
| TCP | flags | `0x02` (SYN) |
| TCP | window | `65535` |
| TCP | payload | `0` |

Evidence:

- `[tcp.header.audit.lengths] frame_len=58 ip_total_len=44 tcp_header_len=24 payload_len=0 tx_len=60 padding=2 ok=1 ...`

## Checksum Table

| Check | Stored | Recomputed | Match | Status |
|---|---:|---:|---:|---|
| IPv4 header checksum | `0x62BC` | `0x62BC` | 1 | ok |
| TCP checksum | `0x7974` | `0x7974` | 1 | ok |

Evidence:

- `[tcp.header.audit.ip] total_len=44 ihl=20 proto=6 checksum=0x62BC recomputed=0x62BC match=1 ok=1 ...`
- `[tcp.header.audit.tcp] src_port=49153 dst_port=18080 data_offset=24 flags=0x02 checksum=0x7974 recomputed=0x7974 match=1 ok=1 ...`

## TX Descriptor / Offload Table

| Field | Value | Status |
|---|---:|---|
| eop | 1 | ok |
| ifcs | 1 | ok |
| rs | 1 | ok |
| checksum_offload | 0 | ok |
| cso | 0 | ok |
| css | 0 | ok |

Evidence:

- `[tcp.tx.offload.audit] eop=1 ifcs=1 rs=1 checksum_offload=0 cso=0 css=0 ok=1 ...`
- `[tcp.checksum.offload.header.audit.done] ok=1 ip_ok=1 tcp_ok=1 offload_ok=1 final_ack_sent=0 http_sent=0 fake=0`

## Mismatch/Fix

Initial audit run found IPv4 recompute mismatch in audit path only (`match=0`).
Fix applied: zeroed full IPv4 checksum word in independent recompute path before summation.
No transmit-path format change required; validation now matches stored checksum.

## TCP Response Truth

- `[tcp.syn.send.retry.proof] attempts=3 sent=1 tx_dd=1 synack_seen=0 rst_seen=0 ... ok=1 ...`
- `[tcp.guest.host.10_0_2_2.probe.done] ... synack_seen=0 rst_seen=0 final_ack_sent=0 http_sent=0 ok=1 ...`

Interpretation:

- SYN frame build/checksum/offload invariants are clean.
- TCP no-response blocker remains external to this audited packet-shape path (environment/backend behavior still dominant suspect).

## Fault Count

- `faults_zero PASS` (0 faults)

## Next

- Since checksum/header/offload are clean: **`QEMU_SLIRP_TCP_LIMITATION_FREEZE_V1`** or **`TAP_HOST_ENV_FIX_PLAN_V1`**.
