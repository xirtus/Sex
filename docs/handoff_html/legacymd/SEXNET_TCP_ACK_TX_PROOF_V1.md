# SEXNET_TCP_ACK_TX_PROOF_V1

Date: 2026-05-19
Phase: G (Task 34)
Status: PASS IMPLEMENTED

## Goal

Send final TCP ACK only if SYN-ACK was validated. Prove ACK build, checksum, TX, and state transition to ESTABLISHED.

## Implementation

After SYN-ACK validation inside the TCP RX handler:
1. Read src MAC from RX frame for Ethernet reply dst
2. Build Ethernet + IPv4 + TCP ACK in TX frame buffer
3. Compute IPv4 checksum
4. Compute TCP checksum (pseudo-header + ACK header)
5. Write TX descriptor 6 (offset 96)
6. Post TDT=7
7. Poll DD (bounded, max 50M)
8. Set state to ESTABLISHED

## ACK Fields

| Field | Value | Note |
|-------|-------|------|
| seq | local_seq + 1 (43) | Acknowledges our SYN |
| ack | remote_seq + 1 | Acknowledges remote SYN |
| flags | ACK | Standard TCP ACK |
| data_offset | 5 | 20-byte header |
| window | 65535 | Maximum window |
| payload | none | Zero-length ACK |

## Positive Markers

| Marker | Description |
|--------|-------------|
| `[sexnet.tcp.ack.build]` | ACK built: seq ack flags=ACK ok=1 |
| `[sexnet.tcp.ack.checksum]` | ACK checksum computed ok=1 |
| `[sexnet.eth.tx.tcp_ack.desc]` | Ethernet TX descriptor for ACK written ok=1 |
| `[sexnet.tcp.ack.tx.poll.done]` | DD poll result: dd_set=1 ok=1 |
| `[sexnet.tcp.ack.tx.proof.done]` | ACK TX proof done: ack_sent=1 tx_dd=1 ok=1 |
| `[sexnet.tcp.handshake.state]` | State transition: state=ESTABLISHED ok=1 |

## Honest Non-PASS

If SYN-ACK not observed, ACK is never sent. The gate must check for presence of ACK markers or honest SKIP reason.

## Source

sexnet source=3.

## File

- `servers/sexnet/src/main.rs` — ACK TX code inside TCP RX handler
