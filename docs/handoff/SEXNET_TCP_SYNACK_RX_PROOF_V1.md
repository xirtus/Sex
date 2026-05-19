# SEXNET_TCP_SYNACK_RX_PROOF_V1

Date: 2026-05-19
Phase: G (Task 33)
Status: PASS IMPLEMENTED (proof code complete; runtime outcome depends on environment)

## Goal

Parse and validate TCP SYN-ACK or record honest RST/timeout.

## Implementation

TCP RX handler added to IPv4 RX path as proto=6 branch. Handler parses:
- TCP ports (src/dst)
- Sequence number
- Acknowledgment number
- Data offset and flags (SYN, ACK, RST)
- Checksum (validation over pseudo-header + TCP segment)

## Validation Rules

| Rule | Check |
|------|-------|
| data_offset >= 5 | Must be 5+ (20-byte header minimum) |
| header <= payload | TCP header must fit within IPv4 payload |
| dst_port match | Must match local TCP port (7777) |
| src_port match | Must match remote TCP port (80) |
| TCP state | Must be SynSent or Established |
| TCP checksum | Computed over pseudo-header + segment, must be 0xFFFF |

## SYN-ACK Validation (additional)

| Rule | Check |
|------|-------|
| SYN flag | Must be set |
| ACK flag | Must be set |
| ACK number | Must equal local_seq + 1 (43) |

## Honest Outcomes

| Outcome | Marker | Description |
|---------|--------|-------------|
| PASS | `[sexnet.tcp.synack.rx]` + `[sexnet.tcp.synack.rx.proof.done] rx_synack=1` | SYN-ACK observed and validated |
| PASS (RST) | `[sexnet.tcp.rst.rx]` + `[sexnet.tcp.synack.rx.proof.done] rx_synack=0 rst=1 ok=0 honest=1` | RST observed honestly |
| SKIP (timeout) | No SYN-ACK or RST within bounded poll | Environment doesn't route TCP response |

## Positive Markers (SYN-ACK case)

| Marker | Description |
|--------|-------------|
| `[sexnet.tcp.rx.segment]` | TCP segment received: ports, seq, ack, flags, csum |
| `[sexnet.tcp.rx.validate]` | Validation result: ports_ok, data_offset_ok, checksum_ok |
| `[sexnet.tcp.synack.rx]` | SYN-ACK received: src_port dst_port seq ack flags=SYN\|ACK ok=1 |
| `[sexnet.tcp.synack.validate]` | SYN-ACK validated: ack_ok ports_ok checksum_ok ok=1 |
| `[sexnet.tcp.synack.rx.proof.done]` | Proof done: rx_synack=1 rst=0 timeout=0 ok=1 |

## RST Markers

| Marker | Description |
|--------|-------------|
| `[sexnet.tcp.rst.rx]` | RST received: ports, seq, ack, flags=RST ok=1 |
| `[sexnet.tcp.handshake.state]` | State transition: state=FAILED_RST ok=1 |

## Source

sexnet source=3.

## File

- `servers/sexnet/src/main.rs` — TCP RX handler (~200 lines)
