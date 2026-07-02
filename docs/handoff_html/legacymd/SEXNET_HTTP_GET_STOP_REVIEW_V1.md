# SEXNET_HTTP_GET_STOP_REVIEW_V1

1. Is sexnet source=3 ESTABLISHED proven?
- Yes. Marker present in source and prior proof logs: `[sexnet.tcp.handshake.state] state=ESTABLISHED ok=1`.

2. Is TCP payload TX proven?
- Yes. Marker present in source and prior proof logs: `[sexnet.tcp.payload.tx.proof.done] sent=1 tx_dd=1 ok=1`.

3. Where should HTTP GET be built?
- In `servers/sexnet/src/main.rs`, inside the existing ESTABLISHED payload TX path (source=3 only).

4. What buffer holds response prefix?
- `HTTP_RESPONSE_BUF` (bounded static buffer in `sexnet`).

5. What max response/body bytes are allowed?
- Response prefix cap: 512 bytes.
- Body prefix cap: 256 bytes.

6. Can Phase I complete without browser route?
- Yes. Phase I is sexnet source=3 TCP payload lane only.

7. What must remain deferred?
- Browser route integration, DNS/TLS, streaming parser, socket API, HAL NET_DIAG retirement, Phase J replacement work.

[sexnet.phaseI.stop_review.pass]
