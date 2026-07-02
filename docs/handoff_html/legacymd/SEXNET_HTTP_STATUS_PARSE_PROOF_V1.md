# SEXNET_HTTP_STATUS_PARSE_PROOF_V1

Date: 2026-05-19

Implemented bounded HTTP status parse upgrade in `servers/sexnet/src/main.rs`.

## Parser Contract
- Accept: `HTTP/1.0 200 ...` and `HTTP/1.1 200 ...`
- Accept line ending: CRLF or LF
- Require: `HTTP/1.` + (`0` or `1`) + space + exactly 3 status digits
- Bound status-line scan: max 128 bytes
- Reject malformed prefix/version/digits without overread

## New Markers
Success:
- `[sexnet.http.status.parse] version=HTTP/1.0|HTTP/1.1 status=NNN line_len=N ok=1`
- `[sexnet.http.status.proof.done] status=NNN ok=1`

Reject:
- `[sexnet.http.status.reject] reason=... ok=1`
- `[sexnet.http.status.proof.done] status=0 ok=0 reason=...`

Diagnostics:
- `[sexnet.http.response.peek.hex] len=N bytes=...` (N<=64)
- `[sexnet.http.response.peek.ascii] len=N text=...` (N<=64)
- `[sexnet.http.response.offset] tcp_payload_offset=N payload_len=N ok=1`

## Current Proof Classification
`PASS REVIEW ONLY`

Reason: this host run (`/tmp/sexnet_http_status_parse_fix.log`) stayed on env-limited lane
(no ESTABLISHED), so source=3 HTTP response bytes were not produced in this boot.

## 2026-05-19 RX Feed Integrity Note
Status parser contract is unchanged/strict. The feed into parser is now hardened to exclude ACK-only frames and non-payload bytes before parse.
