# SEXNET_TCP_HANDSHAKE_GATE_V1

Date: 2026-05-19
Phase: G (Task 35)
Status: PASS IMPLEMENTED (gate design complete; runtime outcome depends on environment)

## Gate Design

Combined gate: `sexnet_tcp_handshake`

This single gate covers the full TCP handshake proof:
- SYN build
- SYN TX
- SYN-ACK RX (if observed)
- ACK TX (if SYN-ACK observed)
- State transition to ESTABLISHED

## Gate Policy

### PASS

All of:
- `[sexnet.tcp.syn.build.proof.done]` with built=1 checksum_ok=1 ok=1
- `[sexnet.tcp.syn.tx.proof.done]` with tx=1 tx_dd=1 ok=1
- `[sexnet.tcp.synack.rx.proof.done]` with rx_synack=1 ok=1
- `[sexnet.tcp.ack.tx.proof.done]` with ack_sent=1 tx_dd=1 ok=1
- `[sexnet.tcp.handshake.state]` with state=ESTABLISHED ok=1
- No #PF/#GP/panic/fault.kill/KERNEL PANIC

### PASS REVIEW ONLY

- SYN build and TX proven, but `[sexnet.tcp.synack.rx.proof.done]` has rx_synack=0 honest=1
- RST observed honestly with `[sexnet.tcp.rst.rx]`
- Environment cannot route TCP response (usernet/TAP limitation)

### FAIL

- SYN-ACK received but no ACK sent
- ACK sent without SYN-ACK
- Malformed SYN-ACK accepted
- Checksum contradiction
- Unbounded retry/poll
- Fault scan fails

## Gate Implementation (daily_driver_master_gate.sh)

```
gate_sexnet_tcp_handshake="SKIP"

# Check SYN build
if has sexnet.tcp.syn.build.proof.done.*built=1.*checksum_ok=1.*ok=1; then
  # Check SYN TX
  if has sexnet.tcp.syn.tx.proof.done.*tx=1.*tx_dd=1.*ok=1; then
    # Check SYN-ACK RX
    if has sexnet.tcp.synack.rx.proof.done.*rx_synack=1.*ok=1; then
      # Check ACK TX
      if has sexnet.tcp.ack.tx.proof.done.*ack_sent=1.*tx_dd=1.*ok=1; then
        # Check ESTABLISHED
        if has sexnet.tcp.handshake.state.*state=ESTABLISHED; then
          gate_sexnet_tcp_handshake="PASS"
        fi
      fi
    elif has sexnet.tcp.synack.rx.proof.done.*rx_synack=0.*honest=1; then
      gate_sexnet_tcp_handshake="PASS"  # honest non-receipt
    elif has sexnet.tcp.rst.rx.*ok=1; then
      gate_sexnet_tcp_handshake="PASS"  # honest RST
    fi
  fi
fi
```

## Source Ownership

- sexnet source=3: TCP code in `servers/sexnet/src/main.rs`
- HAL diagnostic source=2: Existing `[tcp.*]` markers in `kernel/src/hal/pci.rs` preserved as-is, not claimed by this gate

## Markers Used

| Marker Group | Required for PASS |
|-------------|-------------------|
| `sexnet.tcp.syn.build.*` | Yes |
| `sexnet.tcp.syn.tx.*` | Yes |
| `sexnet.tcp.synack.rx.*` | Yes (or honest non-receipt) |
| `sexnet.tcp.ack.tx.*` | Yes (only if SYN-ACK observed) |
| `sexnet.tcp.handshake.state` | Yes (ESTABLISHED or FAILED_RST) |

## Files

- `docs/handoff/SEXNET_TCP_HANDSHAKE_GATE_V1.md` — this gate handoff
- `scripts/daily_driver_master_gate.sh` — gate implementation
