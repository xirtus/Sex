# SEXNET_HTTP_RESPONSE_RX_PROOF_V1

Implemented bounded response-prefix RX in source=3 lane.

- RX scans bounded descriptor/iteration windows.
- Copies TCP payload bytes into static `HTTP_RESPONSE_BUF` cap 512.

Proof markers:
- `[sexnet.http.response.rx] bytes=N bounded=1 ok=1|0`
- `[sexnet.http.response.rx.proof.done] received=1|0 bytes=N ok=1|0`

If peer sends no payload, result is honest non-pass (`received=0`).
