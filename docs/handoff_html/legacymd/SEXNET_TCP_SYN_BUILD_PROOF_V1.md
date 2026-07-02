# SEXNET_TCP_SYN_BUILD_PROOF_V1

Date: 2026-05-19
Phase: G (Task 31)
Status: PASS IMPLEMENTED

## Goal

Prove TCP SYN build with correct header fields, flags, and checksum.

## Implementation

TCP SYN is built proactively in `servers/sexnet/src/main.rs` between L2 proof completion and IPv4 RX entry. The SYN packet is constructed directly in the TX frame buffer with:

- Ethernet header: src=NIC MAC, dst=broadcast
- IPv4 header: src=10.0.2.15, dst=10.0.2.2 (gateway), proto=6, total_len=40
- TCP header: src_port=7777, dst_port=80, seq=42, ack=0, data_offset=5, flags=SYN, window=65535

## Checksum

TCP checksum computed over IPv4 pseudo-header + TCP header:
- Pseudo-header: src IP + dst IP + zero + proto=6 + TCP length (20)
- TCP header: 20 bytes (no options, no payload)
- Standard one's-complement sum with carry folding

## Positive Markers

| Marker | Description |
|--------|-------------|
| `[sexnet.tcp.entry]` | TCP entry with state=CLOSED, local_port, remote IP:port |
| `[sexnet.tcp.syn.build]` | SYN built: src_port=7777 dst_port=80 seq=42 flags=SYN data_offset=5 window=65535 ok=1 |
| `[sexnet.tcp.syn.checksum]` | TCP checksum computed ok=1 |
| `[sexnet.ipv4.tx.tcp_syn.build]` | IPv4 header built: src=10.0.2.15 dst=10.0.2.2 total_len=40 checksum=ok ok=1 |
| `[sexnet.tcp.syn.build.proof.done]` | SYN build proof complete: built=1 checksum_ok=1 ok=1 |

## Negative/Source Audit

- data_offset < 5: rejected by bounds check (guaranteed value is 5)
- TCP header length over frame: bounded by total_len check in IPv4 parse
- Source: sexnet source=3 (not HAL diagnostic source=2)

## File

- `servers/sexnet/src/main.rs` — TCP SYN build code (~120 lines)
