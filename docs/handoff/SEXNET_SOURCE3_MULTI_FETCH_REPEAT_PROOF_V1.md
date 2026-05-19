# SEXNET_SOURCE3_MULTI_FETCH_REPEAT_PROOF_V1

Date: 2026-05-19
Branch: master
Task: 62 — Phase M source3 multi-fetch repeat proof

## Goal

Prove N repeated source3 HTTP GETs succeed with consistent results. N=3 for Phase M V1.

## Method

After the first source3 HTTP GET (Phase I proven path), execute a bounded loop of N=3 total iterations:
1. Each iteration resets TCP state to Closed
2. Performs fresh TCP handshake (SYN→SYN-ACK→final ACK)
3. On ESTABLISHED: builds HTTP GET, transmits via TX descriptor, polls DD
4. Polls RX ring for response, copies payload
5. Parses HTTP status line
6. Extracts body prefix (13 bytes "hello sexnet\n")
7. Emits per-iteration markers

## Markers

```
[sexnet.source3.multi_fetch.begin] target=3 ok=1
[sexnet.source3.multi_fetch.iter] idx=0 status=200 body_len=13 tx_dd=1 rx_bytes=71 ok=1
[sexnet.source3.multi_fetch.iter] idx=1 status=200 body_len=13 tx_dd=1 rx_bytes=71 ok=1
[sexnet.source3.multi_fetch.iter] idx=2 status=200 body_len=13 tx_dd=1 rx_bytes=71 ok=1
[sexnet.source3.multi_fetch.done] attempts=3 success=3 fail=0 ok=1
```

## Rules

- Each iteration is bounded (TX DD poll cap 50M, RX poll cap 1M).
- Reuses existing source3 HTTP path primitives.
- Fresh TCP connection per iteration (reset state to Closed, full handshake).
- Persistent connection not required.
- If host peer absent: SKIP honestly (markers show fail>0).
- No fake success from cached body.

## Classification

PASS IMPLEMENTED when N=3 iterations all show status=200, body_len=13, tx_dd=1.

PASS REVIEW ONLY when markers defined but environment-limited proof run shows honest SKIP.
