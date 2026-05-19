# SEXNET_TCP_PSH_ACK_TX_PROOF_V1

Date: 2026-05-19
Phase: H (Task 37)
Status: SKIP / ENV-BLOCKED (guard proven; payload TX not attempted because !ESTABLISHED)

## Proof Scope

This proof covers:
- TCP payload TX guard (must check state==ESTABLISHED before sending)
- PSH+ACK header build contract (spec only; not exercised in env-blocked state)
- Honest guard failure markers when state!=ESTABLISHED

This proof does NOT cover:
- Actual PSH+ACK payload transmission (requires ESTABLISHED)
- Payload checksum validation
- TX DD confirmation for payload frame

## Guard Implementation

The payload TX guard is implemented in `servers/sexnet/src/main.rs` at the
end of the NIC work block. After the IPv4 RX poll completes, the guard:

1. Locks TCP_STATE
2. Reads current state
3. If state==ESTABLISHED: emits `[sexnet.tcp.payload.tx.guard] state=ESTABLISHED ok=1`
4. If state!=ESTABLISHED: emits `[sexnet.tcp.payload.tx.guard] state=<name> ok=0 reason=not_established`
5. Emits `[sexnet.tcp.payload.proof.done]` with honest status

No PSH+ACK frame is built or transmitted when the guard blocks.

## Required Markers

### Guard pass (if ESTABLISHED is reached in future):
```
[sexnet.tcp.payload.tx.guard] state=ESTABLISHED ok=1
[sexnet.tcp.psh_ack.build] seq=S ack=A payload_len=N flags=PSH|ACK ok=1
[sexnet.tcp.psh_ack.checksum] checksum=0x.... ok=1
[sexnet.ipv4.tx.tcp_payload.build] total_len=N checksum=ok ok=1
[sexnet.eth.tx.tcp_payload.desc] len=N ok=1
[sexnet.tcp.psh_ack.tx.poll.done] dd_set=1 ok=1
[sexnet.tcp.payload.tx.proof.done] sent=1 tx_dd=1 ok=1
```

### Guard block (current env-blocked state):
```
[sexnet.tcp.payload.tx.guard] state=SYN_SENT ok=0 reason=not_established
[sexnet.tcp.payload.proof.done] established=0 payload_tx=0 ... ok=1 reason=guard_blocked_not_established
```

## PSH+ACK Payload Contract (for future unblocked runs)

When ESTABLISHED is reached, the following contract governs PSH+ACK TX:

| Parameter | Value | Notes |
|-----------|-------|-------|
| Payload | "sexnet-phase-h" | 13 bytes, bounded |
| TCP flags | PSH\|ACK (0x18) | PSH=1, ACK=1 |
| SEQ | local_seq + 1 | 43 (local_seq=42 + 1) |
| ACK | remote_seq + 1 | From SYN-ACK seq + 1 |
| data_offset | 5 | 20-byte header, no options |
| window | 65535 | Maximum |
| TCP checksum | Computed over pseudo-header + TCP header + payload | Standard TCP |
| IPv4 total_len | 20 + 20 + payload_len | = 53 bytes |
| TX descriptor | desc 7 (offset 112) | TDT=8 |
| DD poll | bounded 50M iterations | Same as existing |
| Max retries | 1 (no retransmit loop) | One-shot send |

## Current Runtime Evidence

Proof run: `/tmp/sexnet_phase_h_user.log`
QEMU backend: user (SLiRP), e1000e NIC

```
[sexnet.tcp.syn.build.proof.done] built=1 checksum_ok=1 ok=1
[sexnet.tcp.syn.tx.proof.done] tx=1 tx_dd=1 ok=1
[sexnet.tcp.handshake.state] state=SYN_SENT ok=1
```

No SYN-ACK received. State never reaches ESTABLISHED. Payload guard correctly blocks.

## Conclusion

**[sexnet.tcp.payload.tx.proof.done] sent=0 ok=0 honest=1 reason=guard_blocked_not_established**

The payload TX guard is implemented and proven. It correctly prevents TCP payload
transmission when state!=ESTABLISHED. Actual PSH+ACK TX is deferred until an
environment can establish a TCP connection.

## Source Ownership

- sexnet source=3: payload guard code in `servers/sexnet/src/main.rs`
- HAL diagnostic source=2: not involved in Phase H

## Files

- `servers/sexnet/src/main.rs` — payload guard implementation
- `docs/handoff/SEXNET_TCP_PSH_ACK_TX_PROOF_V1.md` — this proof doc
- `docs/handoff/SEXNET_TCP_PAYLOAD_TX_STOP_REVIEW_V1.md` — STOP review


## 2026-05-19 PSH+ACK Wire-Shape Patch
- Implemented TX tail wrap fix for desc7 payload post: publish `TDT=0` (ring wrap) instead of `TDT=8`.
- Added bounded payload/shape diagnostics:
  - `[sexnet.tcp.psh_ack.shape]`
  - `[sexnet.tcp.psh_ack.payload.peek.hex]`
  - `[sexnet.tcp.psh_ack.payload.peek.ascii]`
  - `[sexnet.tcp.psh_ack.ack_expect]`
  - `[sexnet.tcp.psh_ack.peer_ack]`
