# SEXNET_HTTP_TIMEOUT_RETRY_POLICY_PROOF_V1

Date: 2026-05-19
Branch: master
Task: 64 — Phase M HTTP timeout/retry policy proof

## Goal

Document and prove that source3 HTTP retry/timeout behavior is bounded and honest.

## Current Policy

All poll-based operations in sexnet source3 use hard iteration caps:

| Operation | Max Iterations | Marker |
|-----------|---------------|--------|
| TX DD poll | 50,000,000 | `sexnet.tcp.psh_ack.tx.poll.done` |
| RX response poll | 1,000,000 | `sexnet.http.response.rx.proof.done` |
| TCP SYN-ACK poll | 128 (NIC scan) | `sexnet.tcp.synack.rx.proof.done` |
| NIC reset poll | 1,000,000 | `sexnet.nic.reset.ctrl.rst.poll` |
| Link poll | 1,000,000 | `sexnet.nic.link.poll.done` |

### Retry Policy

- **No infinite retry**: Every loop has an explicit iteration cap.
- **No unbounded RX poll**: RX poll caps at 1,000,000.
- **Timeout handling**: If SYN-ACK not received within poll window, TCP state → FAILED_TIMEOUT. No silent hangs.
- **RST handling**: If RST received, TCP state → FAILED_RST. Markers report honestly.
- **Success path**: Does NOT require retries; first attempt succeeds if environment is ready.

### Phase M multi-fetch retry

For multi-fetch (N=3 iterations):
- Each iteration attempts fresh TCP handshake
- If handshake fails (timeout/RST), iteration marked as fail
- Loop continues to next iteration
- Total iterations bounded at N=3, no infinite retry

## Markers

```
[sexnet.http.retry.policy] max_attempts=3 timeout_polls=1000000 bounded=1 ok=1
[sexnet.http.retry.iter] attempt=0 result=success ok=1
[sexnet.http.retry.iter] attempt=1 result=success ok=1
[sexnet.http.retry.iter] attempt=2 result=success ok=1
[sexnet.http.retry.proof.done] bounded=1 ok=1
```

## Rules

- No infinite retry.
- No unbounded RX poll.
- Timeout must SKIP/FAIL honestly depending proof mode.
- Success path must not require retries (first attempt succeeds if env ready).
- Do not hide RST.
- Do not change scheduler/time.

## Classification

PASS IMPLEMENTED when all retry markers show bounded=1 and all attempts result in success/timeout/rst honestly.

If environment-limited: PASS REVIEW ONLY (policy documented, markers implemented, runtime proof deferred).
