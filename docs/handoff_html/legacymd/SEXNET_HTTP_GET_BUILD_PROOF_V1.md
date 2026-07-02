# SEXNET_HTTP_GET_BUILD_PROOF_V1

Implemented bounded HTTP GET builder in `servers/sexnet/src/main.rs`.

- Host: `example.com`
- Path: `/`
- Connection: `close`
- Request format: HTTP/1.1
- Buffer: static `HTTP_GET_BUF` (cap 192)

Proof markers:
- `[sexnet.http.get.build] host=example.com path=/ len=N ok=1`
- `[sexnet.http.get.proof.done] built=1 len=N ok=1`
