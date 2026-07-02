# SEXNET_HTTP_GET_GATE_AND_HANDOFF_V1

Gate: `sexnet_http_get_source3` in `scripts/daily_driver_master_gate.sh`

PASS requires all:
- `[sexnet.phaseI.stop_review.pass]`
- `[sexnet.http.get.proof.done] built=1 ... ok=1`
- `[sexnet.http.get.tx.proof.done] sent=1 tx_dd=1 ok=1`
- `[sexnet.http.response.rx.proof.done] received=1 ... ok=1`
- `[sexnet.http.status.proof.done] status=[1-9][0-9][0-9] ok=1`
- `[sexnet.http.body.proof.done] ... ok=1`
- `[sexnet.phaseI.readiness] ... source=3 ok=1`
- fault scan PASS

FAIL if:
- HTTP TX claimed without ESTABLISHED
- status proof emits malformed/reject (`status=0 ok=0`)
- fault scan fails

SKIP if:
- env-limited/no ESTABLISHED/no peer response/no full source=3 chain

2026-05-19 update:
- parser path now emits explicit status-reject markers and bounded response peek markers
- gate policy remains strict: malformed status never passes

- 2026-05-19 payload RX fix: source=3 response copy now requires ACK=1, RST=0, payload_len>0 from IPv4/TCP header math; ACK-only frames are skipped with explicit marker.

## 2026-05-19 PSH+ACK Shape Notes
- `servers/sexnet/src/main.rs` now emits explicit shape/payload/expectation markers for source3 GET TX:
  - `[sexnet.tcp.psh_ack.shape]`
  - `[sexnet.tcp.psh_ack.payload.peek.hex]`
  - `[sexnet.tcp.psh_ack.payload.peek.ascii]`
  - `[sexnet.tcp.psh_ack.ack_expect]`
  - `[sexnet.tcp.psh_ack.peer_ack]`
- Runtime reproof is pending in a lane that reaches Phase I TCP handshake/payload.
