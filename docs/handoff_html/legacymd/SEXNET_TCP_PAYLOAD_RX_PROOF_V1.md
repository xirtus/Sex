# SEXNET_TCP_PAYLOAD_RX_PROOF_V1

Date: 2026-05-19
Phase: H (Task 38)
Status: SKIP (not attempted — no ESTABLISHED connection, no peer payload)

## Proof Scope

This proof covers:
- TCP payload RX guard (must check state==ESTABLISHED before accepting)
- Honest guard failure when state!=ESTABLISHED

This proof does NOT cover:
- Actual TCP payload receive and validation (requires ESTABLISHED and peer data)
- PSH segment parsing
- Payload bounds checking
- Payload copy from RX buffer
- Received data handoff

## Guard Implementation

The payload RX guard is co-located with the TX guard in the main NIC work block.
It reads TCP_STATE and:

1. If state==ESTABLISHED: would scan RX ring for TCP segments with PSH flag,
   validate checksum, check sequence numbers, extract bounded payload.
2. If state!=ESTABLISHED: emits honest block marker.

## Required Markers

### PASS (if ESTABLISHED and peer sends payload):
```
[sexnet.tcp.payload.rx.guard] state=ESTABLISHED ok=1
[sexnet.tcp.payload.rx.segment] seq=S ack=A payload_len=N flags=... ok=1
[sexnet.tcp.payload.rx.validate] seq_ok=1 ack_ok=1 checksum_ok=1 len_ok=1 ok=1
[sexnet.tcp.payload.rx.copy] copied=N bounded=1 ok=1
[sexnet.tcp.payload.rx.proof.done] received=1 bytes=N ok=1
```

### SKIP (current env-blocked state):
```
[sexnet.tcp.payload.rx.guard] state=SYN_SENT ok=0 reason=not_established
[sexnet.tcp.payload.proof.done] established=0 payload_rx=0 ... ok=1 reason=guard_blocked_not_established
```

## Accepted Outcomes

| Outcome | Condition | Marker |
|---------|-----------|--------|
| A. PASS | ESTABLISHED + peer sends payload | received=1 bytes=N ok=1 |
| B. PASS REVIEW ONLY / SKIP | ESTABLISHED but peer sends no payload | received=0 honest=1 reason=no_peer_payload |
| C. SKIP | Not ESTABLISHED | received=0 ok=1 reason=not_established |
| D. FAIL | Payload bounds violation, malformed segment | received=0 ok=0 reason=... |

Current outcome: **C. SKIP** — not ESTABLISHED.

## Payload RX Contract (for future unblocked runs)

When ESTABLISHED is reached and a peer sends TCP data:

| Rule | Requirement |
|------|-------------|
| Segment parse | Validate TCP data_offset, extract PSH flag, payload length |
| Sequence check | Verify seq is within acceptable window |
| Checksum validate | Full TCP checksum over pseudo-header+header+payload |
| Bounds check | Payload must fit within bounded buffer (max 1460 bytes) |
| Copy | Bounded copy from RX ring pkt_buf to local buffer |
| No streaming | One payload only; no reassembly across segments |
| No HTTP parsing | Payload is opaque bytes |
| No browser feed | Payload is not forwarded to browser |

## Current Runtime Evidence

Proof run: `/tmp/sexnet_phase_h_user.log`

State is SYN_SENT. No ESTABLISHED. No peer payload possible.

## Conclusion

**[sexnet.tcp.payload.rx.proof.done] received=0 honest=1 reason=not_established**

Payload RX is SKIPped because the TCP connection is not ESTABLISHED. The RX guard
correctly prevents payload acceptance. No unsafe code paths are reachable.

## Source Ownership

- sexnet source=3: payload RX guard code in `servers/sexnet/src/main.rs`

## Files

- `servers/sexnet/src/main.rs` — payload RX guard implementation
- `docs/handoff/SEXNET_TCP_PAYLOAD_RX_PROOF_V1.md` — this proof doc

