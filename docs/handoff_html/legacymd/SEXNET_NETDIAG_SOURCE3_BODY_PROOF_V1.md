# SEXNET_NETDIAG_SOURCE3_BODY_PROOF_V1

Date: 2026-05-19
Phase: J (Task 50)
Status: PASS IMPLEMENTED

## Goal

Prove diagnostic body/result content comes from sexnet source=3 HTTP body buffer.

## Body Data Source

The Phase I source=3 HTTP GET path stores response body data in:

| Buffer | Variable | Capacity | Purpose |
|--------|----------|----------|---------|
| HTTP response | `HTTP_RESPONSE_BUF` | 512 bytes | Raw HTTP response |
| Body prefix | `HTTP_BODY_PREFIX_BUF` | 256 bytes | Bounded body capture |
| Body length | `HTTP_BODY_PREFIX_LEN` | — | Actual bytes captured |

Body capture is bounded: max 256 bytes, truncated with explicit marker. No unbounded copy. No file persistence. No HTML parse. No TLS. No browser render.

## Phase I Source=3 HTTP Markers (Prerequisites)

These Phase I markers must be present for body proof to be valid:

```
[sexnet.http.get.tx.proof.done] sent=1 tx_dd=1 ok=1
[sexnet.tcp.psh_ack.peer_ack] ack=N expect_ack=N advanced=1 ok=1
[sexnet.http.response.rx] bytes=N bounded=1 ok=1
[sexnet.http.status.proof.done] status=200 ok=1
[sexnet.http.body.proof.done] bytes=N ok=1
[sexnet.phaseI.readiness] established=1 payload_tx=1 source=3 ok=1
```

## Phase J Body Proof Markers (Added)

```
[sexnet.netdiag.source3.body] source=3 status=200 body_len=13 bounded=1 ok=1
[sexnet.netdiag.source3.body.proof.done] source=3 body_len=13 ok=1
```

These are emitted in the source=3 HTTP GET success path, after `[sexnet.http.body.proof.done]` and `[sexnet.phaseI.readiness]`, and only when `phase_i_ok == 1`.

## Rules Enforced

| Rule | Status |
|------|--------|
| Do not copy unbounded body | ENFORCED — cap at 256 bytes |
| Body prefix cap fixed/bounded | ENFORCED — `HTTP_BODY_BUF_CAP = 256` |
| No browser render | ENFORCED — no browser grants |
| No file persistence | ENFORCED — stack/static buffers only |
| No HTML parse | ENFORCED — raw bytes only |
| No TLS | ENFORCED — HTTP only |
| No source=2 body accepted as source=3 | ENFORCED — markers use source=3 path only |

## Body Identity Proof

The body captured in `HTTP_BODY_PREFIX_BUF` comes from the TCP PSH+ACK payload received after the source=3 HTTP GET request. The response RX marker chain proves:

1. `[sexnet.http.response.rx]` — bytes were received from TCP payload
2. `[sexnet.http.status.parse]` — HTTP status line was parsed from response
3. `[sexnet.http.body.buffer]` — body bytes were copied to bounded buffer
4. `[sexnet.http.body.proof.done]` — body capture confirmed
5. `[sexnet.netdiag.source3.body]` — body is confirmed source=3 primary

## Doc Marker

```
[sexnet.netdiag.source3.body.proof.done] source=3 body_len=13 ok=1
```
