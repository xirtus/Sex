# SEXNET_HTTP_STATUS_PARSE_PROOF_V1

Implemented status-line only parser for HTTP/1.x response prefix.

Proof markers:
- `[sexnet.http.status.parse] version=HTTP/1.x status=NNN ok=1`
- `[sexnet.http.status.proof.done] status=NNN ok=1`

Malformed/missing response is marked honestly:
- `[sexnet.http.status.parse] status=0 ok=0 reason=malformed_or_missing`
