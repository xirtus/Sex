# SEXNET_HTTP_STATUS_PARSE_FIX_V1

Date: 2026-05-19
Scope: `servers/sexnet/src/main.rs` only

## Old Failure
Previous known proof run (`/tmp/sexnet_phase_i_e1000_reproof.log`) showed:
- `[sexnet.tcp.handshake.state] state=ESTABLISHED ok=1`
- `[sexnet.tcp.payload.tx.proof.done] sent=1 tx_dd=1 ok=1`
- `[sexnet.http.response.rx] bytes=6 bounded=1 ok=1`
- `[sexnet.http.status.proof.done] status=0 ok=0`
- gate: `sexnet_http_get_source3 FAIL` reason `HTTP status parse malformed`

## Response Peek / Observed Bytes
New bounded diagnostics added (up to 64 bytes):
- `[sexnet.http.response.peek.hex] len=N bytes=...`
- `[sexnet.http.response.peek.ascii] len=N text=...`
- `[sexnet.http.response.offset] tcp_payload_offset=N payload_len=N ok=1`

Current reproof in this host lane did not reach ESTABLISHED, so source=3 response bytes were not produced in this run.

## Root Cause
Status parser was too narrow and opaque:
- only CRLF line terminator handling
- generic malformed marker without bounded byte visibility
- no explicit HTTP version or reject reason marker

## Parser Fix
Implemented bounded status-line parse with explicit reject reasons:
- accepts `HTTP/1.0 200 ...` and `HTTP/1.1 200 ...`
- accepts CRLF or LF line end
- requires `HTTP/1.` + (`0` or `1`) + space + exactly 3 status digits
- bounded status-line scan cap: 128 bytes
- never reads beyond received length

Success markers:
- `[sexnet.http.status.parse] version=HTTP/1.0|HTTP/1.1 status=NNN line_len=N ok=1`
- `[sexnet.http.status.proof.done] status=NNN ok=1`

Reject markers:
- `[sexnet.http.status.reject] reason=... ok=1`
- `[sexnet.http.status.proof.done] status=0 ok=0 reason=...`

## Body Boundary
Body extraction now:
- uses `\r\n\r\n` separator when present
- otherwise falls back to post-status-line start (bounded, honest)
- preserves fixed-cap body buffer

Markers:
- `[sexnet.http.body.buffer] bytes=N cap=N truncated=T ok=1`
- `[sexnet.http.body.proof.done] bytes=N ok=1`

## Proof Result (This Run)
- Build: PASS
- Fault scan: PASS (`faults_zero PASS`)
- `sexnet_http_phase_i_readiness`: SKIP (env-limited, no ESTABLISHED in this boot)
- `sexnet_http_get_source3`: SKIP (no source=3 response path reached)
- Remaining unrelated fail: `sexnet_nic_tx_frame_observe` (pre-existing)

Classification: `PASS REVIEW ONLY`

## Files Changed
- `servers/sexnet/src/main.rs`
- `docs/handoff/SEXNET_HTTP_STATUS_PARSE_FIX_V1.md`
- `docs/handoff/SEXNET_HTTP_STATUS_PARSE_PROOF_V1.md`
- `docs/handoff/SEXNET_HTTP_GET_GATE_AND_HANDOFF_V1.md`
- `docs/handoff/NETWORK_STACK_STATUS_ROLLUP_V1.md`
- `docs/handoff/NETWORK_SPRINT_EXECUTION_V1.md`
