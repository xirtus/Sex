# SEXNET_UDP_ECHO_REPLY_PROOF_V1

Date: 2026-05-19
Branch: master
Depends: SEXNET_UDP_HEADER_VALIDATE_PROOF_V1

## Goal

Build and send UDP echo reply from a validated UDP datagram received on IPv4 proto=17.

## Positive Markers

| Marker | Fields | Meaning |
|--------|--------|---------|
| `[sexnet.udp.tx.reply.build]` | src_port=P dst_port=P len=N payload_len=N ok=1 | Echo reply UDP header built |
| `[sexnet.udp.tx.reply.checksum]` | checksum=0x0000 policy=zero_allowed ok=1 | TX checksum set to zero (no checksum) |
| `[sexnet.ipv4.tx.udp_reply.build]` | src=10.0.2.15 dst=A.B.C.D total_len=N checksum=ok ok=1 | IPv4 reply header built with correct checksum |
| `[sexnet.eth.tx.udp_reply.desc]` | len=N ok=1 | Ethernet TX descriptor written |
| `[sexnet.udp.tx.poll.done]` | dd_set=1 ok=1 | Hardware consumed TX descriptor |
| `[sexnet.udp.echo.proof.done]` | rx_udp=1 tx_reply=1 tx_dd=1 ok=1 | Full echo reply proof complete |

## Echo Reply Rules

1. **Swap ports**: reply src_port = request dst_port; reply dst_port = request src_port
2. **Same payload**: echo the exact payload bytes from the request (bounded by RX buffer)
3. **Checksum**: set to 0 (no checksum, valid per RFC 768 for IPv4)
4. **Max payload**: bounded by RX frame buffer (static allocation, ~2048 bytes)
5. **No port registry**: any UDP port is served
6. **No DNS interpretation**: payload treated as opaque bytes
7. **No heap allocation**: all stack/frame buffer based
8. **No socket abstraction**: direct buffer manipulation

## Reply Construction Chain

```
RX: Ethernet → IPv4(proto=17) → UDP
           ↓
TX: Ethernet ← IPv4(proto=17, src=10.0.2.15, dst=request.src) ← UDP(ports swapped, payload echoed)
           ↓
      TX desc 4 (offset 64, TDT=5) → DD poll → done
```

## Proof Acceptance

Echo reply proof is accepted if:
- `[sexnet.udp.echo.proof.done]` with `tx_dd=1 ok=1` appears
- All intermediate markers present
- No faults in log
