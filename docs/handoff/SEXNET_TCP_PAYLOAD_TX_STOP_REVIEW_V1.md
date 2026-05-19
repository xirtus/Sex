# SEXNET_TCP_PAYLOAD_TX_STOP_REVIEW_V1

Date: 2026-05-19
Phase: H (TCP payload)
Review: STOP review before TCP payload implementation

## STOP Review Questions

### 1. Does the current sexnet source=3 TCP path ever reach ESTABLISHED in runtime?

**Answer:** No. Current usernet environment does not deliver a SYN-ACK to the guest.
The sexnet source=3 TCP handshake sends SYN (TX DD confirmed), sets state=SYN_SENT,
but no SYN-ACK is received from the gateway (10.0.2.2:80). The usernet SLiRP backend
does not forward TCP connections initiated from guest to host ports unless configured
with hostfwd. No hostfwd is configured in the current proof profile.

Evidence:
- `[sexnet.tcp.syn.tx.proof.done] tx=1 tx_dd=1 ok=1` (SYN sent)
- `[sexnet.tcp.handshake.state] state=SYN_SENT ok=1` (never reaches ESTABLISHED)
- No `[sexnet.tcp.synack.rx]` marker observed
- No `[sexnet.tcp.handshake.state] state=ESTABLISHED` marker observed

### 2. Does a log show validated SYN-ACK and final ACK TX?

**Answer:** No. SYN-ACK is never received in the current usernet proof run.
Final ACK is never built or transmitted.

### 3. Is there a safe peer for payload exchange, e.g. host listener, TAP, or usernet hostfwd?

**Answer:** Not currently configured. Options:
- **usernet hostfwd**: QEMU `hostfwd=tcp::8080-:7777` would forward host TCP to guest.
  Not enabled in current proof profile. Adding it would require proof profile changes,
  which exceeds Phase H scope.
- **TAP**: A TAP interface with a host TCP listener on 10.0.2.15:7777 could respond
  with SYN-ACK. TAP requires root/CAP_NET_RAW, not available in all environments.
- **nc listener**: Could be started pre-QEMU to accept on port 80 and respond with
  raw SYN-ACK bytes, but this is complex and fragile.

### 4. Is there already TCP payload TX code?

**Answer:** No. Only a payload guard exists (added in Phase H STOP review). The guard
checks TCP state and prevents any payload TX unless state==ESTABLISHED. No PSH/ACK
build, no payload copy, no TCP data TX descriptor exists.

### 5. Is there already TCP payload RX code?

**Answer:** No. The TCP RX path (Phase G) only handles SYN-ACK and RST flags during
the handshake. It does not parse PSH flag, does not extract payload from TCP segments,
does not buffer received data.

### 6. Is there existing FIN/RST handling?

**Answer:** Partial. RST handling exists in Phase G: if a TCP segment with RST flag
is received during SYN_SENT or ESTABLISHED state, the state transitions to FAILED_RST.
No FIN handling exists. No close/cleanup on FIN exists.

### 7. Can Phase H complete without HTTP/browser/socket API?

**Answer:** Yes. Phase H scope is explicitly:
- TCP payload TX guard (must check state==ESTABLISHED)
- PSH+ACK TX (only if ESTABLISHED)
- Payload RX (only if ESTABLISHED and peer sends)
- FIN/RST handling
- No HTTP parsing
- No browser networking
- No socket API
- No streaming

### 8. Can this be done without kernel/ABI/sex-pdx edits?

**Answer:** Yes. The payload guard, PSH+ACK TX (if reachable), payload RX, and
FIN/RST handling can all be added within `servers/sexnet/src/main.rs` using
existing infrastructure (NIC TX descriptors, IPv4 RX path, TX frame buffer).

### 9. What STOP FIRST boundaries apply?

**Answer:**
- **No kernel edits**
- **No sex-pdx/global ABI edits**
- **No scheduler/PKRU/time changes**
- **No browser/sexdisplay/shell changes**
- **No HTTP** — payload is opaque bounded bytes, not HTTP
- **No general socket API**
- **No TCP streaming** — one bounded payload only
- **No multi-connection table** — one connection only
- **No source=3 migration** — new code is source=3 in sexnet
- **HAL NET_DIAG retirement** — deferred
- **All polls bounded** — max 50M iterations per DD poll
- **Payload must be bounded** — fixed 13-byte payload "sexnet-phase-h"
- **No payload sent before ESTABLISHED** — enforced by guard

## STOP Review Conclusion

**[sexnet.phaseH.stop_review.env_blocked reason=no_established_tcp]**

Phase H implementation cannot proceed to full PASS IMPLEMENTED because the
current environment does not establish a TCP connection. The sexnet source=3
TCP handshake sends SYN successfully but never receives SYN-ACK. Without
ESTABLISHED state, TCP payload TX is forbidden by the Phase H contract.

### What IS implemented (guard-only):
- `[sexnet.tcp.payload.tx.guard]` — checks state and blocks TX when !=ESTABLISHED
- `[sexnet.tcp.payload.rx.guard]` — blocks RX when !=ESTABLISHED
- `[sexnet.tcp.fin_rst.guard]` — reports state for close handling
- `[sexnet.tcp.payload.proof.done]` — honest proof wrap-up

### What is NOT implemented (requires ESTABLISHED):
- PSH+ACK header build with payload
- TCP payload checksum over pseudo-header+header+payload
- IPv4 total_len update for TCP data
- Ethernet TX descriptor for TCP payload
- Payload RX segment validation and copy
- FIN flag handling and state transition
- FIN_WAIT/CLOSED state transitions

### What would unblock Phase H:
A. **TAP + host listener**: Configure TAP interface and start `nc -l 7777` to
   accept the SYN and respond with raw SYN-ACK bytes.
B. **usernet hostfwd**: Add `hostfwd=tcp::7777-:7777` to QEMU args and start
   `nc -l 7777` on host.
C. **Real hardware**: Connect to a real network where a peer on 10.0.2.2:80
   (or configured remote) responds to TCP SYN.

Each option requires proof profile changes beyond Phase H scope.

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Guard not enforced | Guard is code-checked at compile time; no payload TX path exists without it |
| False ESTABLISHED | Guard reads TCP_STATE atomically; state only set on validated SYN-ACK |
| Payload sent after RST | Guard checks state; FAILED_RST blocks payload |
| Unbounded payload | Fixed 13-byte payload "sexnet-phase-h" in contract; not yet implemented |
| Fault on guard read | State is a Mutex-protected enum; read is safe |

## File Plan

| File | Change |
|------|--------|
| `servers/sexnet/src/main.rs` | ADDED: TCP payload guard section |
| `docs/handoff/SEXNET_TCP_PAYLOAD_TX_STOP_REVIEW_V1.md` | CREATE (this file) |
| `docs/handoff/SEXNET_TCP_PSH_ACK_TX_PROOF_V1.md` | CREATE |
| `docs/handoff/SEXNET_TCP_PAYLOAD_RX_PROOF_V1.md` | CREATE |
| `docs/handoff/SEXNET_TCP_FIN_RST_HANDLING_PROOF_V1.md` | CREATE |
| `docs/handoff/SEXNET_TCP_PAYLOAD_GATE_AND_HANDOFF_V1.md` | CREATE |
| `docs/handoff/NETWORK_STACK_STATUS_ROLLUP_V1.md` | UPDATE |
| `scripts/daily_driver_master_gate.sh` | UPDATE: add sexnet_tcp_payload gate |

