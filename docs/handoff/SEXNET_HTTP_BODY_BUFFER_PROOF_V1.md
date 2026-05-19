# SEXNET_HTTP_BODY_BUFFER_PROOF_V1

Implemented bounded body-prefix extraction from response prefix.

- Buffer: `HTTP_BODY_PREFIX_BUF`
- Cap: 256 bytes
- No browser, no HTML parsing, no persistence

Proof markers:
- `[sexnet.http.body.buffer] bytes=N cap=256 truncated=T ok=1`
- `[sexnet.http.body.proof.done] bytes=N ok=1`
