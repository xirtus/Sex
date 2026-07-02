# SEXNET_TCP_SYN_TX_PROOF_V1

Date: 2026-05-19
Phase: G (Task 32)
Status: PASS IMPLEMENTED

## Goal

Prove TCP SYN transmission over existing IPv4/Ethernet TX path.

## Implementation

After SYN build, the frame is padded to 60 bytes (minimum Ethernet frame), written to TX descriptor 5 (offset 80), TDT posted to 6, and DD polled with bounded loop (max 50M iterations). On DD=1, TCP state transitions to SYN_SENT.

## TX Path

- TX descriptor: slot 5 (offset 80 from TX_PERM_DESC_VA)
- Frame buffer: TX_PERM_FRAME_VA (shared, sequential use)
- TDT post: nic_va + 0x3818 ← 6
- DD poll: max 50M iterations per standard TX pattern
- No TCP payload
- No HTTP
- No browser path

## Positive Markers

| Marker | Description |
|--------|-------------|
| `[sexnet.eth.tx.tcp_syn.desc]` | Ethernet TX descriptor written, len=60 ok=1 |
| `[sexnet.tcp.syn.tx.post]` | TDT posted, slot=6 ok=1 |
| `[sexnet.tcp.syn.tx.poll.done]` | DD poll result: dd_set=1 ok=1 |
| `[sexnet.tcp.syn.tx.proof.done]` | SYN TX proof complete: tx=1 tx_dd=1 ok=1 |
| `[sexnet.tcp.handshake.state]` | State transition: state=SYN_SENT ok=1 |

## Bounded Retries

Single SYN send for Phase G. Bounded retry (max 3) available for future phases. Current implementation sends exactly one SYN and awaits response.

## Source

sexnet source=3 (TCP code in sexnet server, not HAL diagnostic).

## File

- `servers/sexnet/src/main.rs` — TCP SYN TX code
