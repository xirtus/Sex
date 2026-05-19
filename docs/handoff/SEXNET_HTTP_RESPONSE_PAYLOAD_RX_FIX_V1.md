# SEXNET_HTTP_RESPONSE_PAYLOAD_RX_FIX_V1

Date: 2026-05-19
Scope: `servers/sexnet/src/main.rs` only (Phase I source=3 HTTP RX path)

## Old Failure
From prior failing log (`/tmp/sexnet_phase_i_http_peer.log`):
- `[sexnet.http.response.rx] bytes=12 bounded=1 ok=1`
- `[sexnet.http.response.peek.hex] len=12 bytes=00 00 00 00 00 00 00 00 00 00 00 00`
- `[sexnet.http.status.reject] reason=missing_line_ending ok=1`
- `[sexnet.http.status.proof.done] status=0 ok=0 reason=missing_line_ending`

## Root Cause
HTTP RX copy used `rlen - payload_off` from descriptor frame length and accepted any matching TCP segment.
That allowed ACK-only/non-payload segments or non-IP-total payload region bytes to be copied into HTTP buffer.

## Fix Implemented
In Phase I response RX poll:
- parse Ethernet/IPv4/TCP header lengths (`14`, `ihl*4`, `data_offset*4`)
- compute `tcp_payload_offset = 14 + ip_ihl_bytes + tcp_data_offset_bytes`
- compute `tcp_payload_len = ip_total_len - ip_ihl_bytes - tcp_data_offset_bytes`
- enforce boundedness: `tcp_payload_end <= rlen`
- require response segment policy: `ACK=1`, `RST=0`, `tcp_payload_len > 0`
- skip payloadless ACKs with explicit marker:
  - `[sexnet.http.response.rx.skip] reason=no_tcp_payload flags=... ok=1`
- emit payload offset marker only on real payload:
  - `[sexnet.http.response.offset] tcp_payload_offset=N payload_len=N frame_len=N ok=1`

Copy loop remains fully bounded by payload length and `HTTP_RESPONSE_BUF_CAP`.

## Current Proof Run (this environment)
Log: `/tmp/sexnet_http_response_payload_rx_fix.log`
- `[sexnet.tcp.handshake.state] state=SYN_SENT ok=1`
- `[sexnet.phaseI.readiness] established=0 payload_tx=0 source=3 ok=0`
- `sexnet_http_get_source3`: `SKIP` (env-limited, no ESTABLISHED)
- `faults_zero`: `PASS`
- FINAL: `PASS (248 gates proved, 49 skipped, 0 faults)`

Classification for this turn: `PASS IMPLEMENTED` (fix landed) + `PASS REVIEW ONLY` for live source=3 HTTP payload/status proof in this host lane.
