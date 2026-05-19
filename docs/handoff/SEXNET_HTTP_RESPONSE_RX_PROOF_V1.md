# SEXNET_HTTP_RESPONSE_RX_PROOF_V1

Implemented bounded response-prefix RX in source=3 lane.

- RX scans bounded descriptor/iteration windows.
- Copies TCP payload bytes into static `HTTP_RESPONSE_BUF` cap 512.

Proof markers:
- `[sexnet.http.response.rx] bytes=N bounded=1 ok=1|0`
- `[sexnet.http.response.rx.proof.done] received=1|0 bytes=N ok=1|0`

If peer sends no payload, result is honest non-pass (`received=0`).

## 2026-05-19 Payload-Offset Hardening Update
Phase I RX now copies only real TCP payload based on IPv4 `total_len` and TCP `data_offset`:
- `tcp_payload_offset = 14 + ihl*4 + data_offset*4`
- `tcp_payload_len = ip_total_len - ihl*4 - data_offset*4`
- skip `tcp_payload_len==0` with `[sexnet.http.response.rx.skip] reason=no_tcp_payload ... ok=1`
- bounded check requires payload end within descriptor frame length before copy

New payload marker shape:
- `[sexnet.http.response.offset] tcp_payload_offset=N payload_len=N frame_len=N ok=1`
