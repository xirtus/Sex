# SEXNET_HTTP_PHASE_I_READINESS_GATE_V1

Date: 2026-05-19
Branch: master
Predecessors: SEXNET_PHASE_G_RUNTIME_REPROOF_V1, SEXNET_PHASE_H_RUNTIME_REPROOF_V1, SEXNET_E1000E_NIC_RESET_FOR_RX_V1

## Goal

Determine whether Phase I (HTTP GET) may start.
Do NOT implement HTTP GET yet.
This is a readiness gate document only.

## Readiness Criteria

Phase I readiness PASS requires ALL of:

| # | Criterion | Evidence |
|---|-----------|----------|
| 1 | Phase G runtime: ESTABLISHED proven | `sexnet.tcp.handshake.state.*state=ESTABLISHED ok=1` |
| 2 | Phase H runtime: PSH+ACK payload TX proven after ESTABLISHED | `sexnet.tcp.payload.tx.proof.done.*sent=1.*tx_dd=1 ok=1` |
| 3 | 0 faults | No `#PF`, `#GP`, `panic`, `KERNEL PANIC` in log |
| 4 | No payload before ESTABLISHED | Guard markers confirm block when not established |
| 5 | Source ownership clear: sexnet source=3 | Markers use `sexnet.tcp.*` prefix (not `tcp.*` HAL diag) |

## Current Status

- **Phase I readiness: TBD** (depends on Phase G/H reproof results)
- HTTP GET: NOT IMPLEMENTED
- Browser networking: NOT IMPLEMENTED
- HAL NET_DIAG retirement: deferred

## Readiness Decision Matrix

| Phase G | Phase H | Faults | Source | Decision |
|---------|---------|--------|--------|----------|
| ESTABLISHED | Payload TX | 0 | sexnet source=3 | **YES — Phase I may start** |
| ESTABLISHED | Guard only (no TX) | 0 | sexnet source=3 | NO-GO — payload TX not proven |
| SYN_SENT/RST/Timeout | Guard only | 0 | sexnet source=3 | NO-GO — no ESTABLISHED |
| Any | Any | >0 | any | STOP FIRST — fix faults first |
| Any | Any | Any | HAL diag source=2 only | NO-GO — wrong source ownership |

## Gate Design

### New Gate: sexnet_http_phase_i_readiness

Location: scripts/daily_driver_master_gate.sh

Policy:
- PASS: Phase G ESTABLISHED + Phase H payload TX + 0 faults + source=3
- SKIP: Not ready due to environment (no SYN-ACK, no ESTABLISHED)
- FAIL: Docs/log claim ready but evidence contradicts
- Never PASS based on mock HTTP/browser markers

### Log Marker Accepted

```
[sexnet.phaseI.readiness] established=1 payload_tx=1 source=3 ok=1
```

Or derived from existing Phase G/H markers:
- `sexnet.tcp.handshake.state.*state=ESTABLISHED ok=1`
- `sexnet.tcp.payload.tx.proof.done.*sent=1.*tx_dd=1 ok=1`
- Combined with fault scan (0 faults)

## What Phase I Will Include (When Ready)

- HTTP GET request build (method, host, path, headers)
- HTTP response parse (status line, headers, body)
- Single bounded request/response cycle
- TX via existing TCP infrastructure (desc 7/8)
- No browser integration
- No persistent connections
- No chunked encoding
- No redirect following

## What Phase I Will NOT Include

- Browser networking
- Multi-page browsing
- HTTPS/TLS
- WebSocket
- Cookie/store
- CSS/JS parsing
- HTML rendering

## Markers

- [sexnet.phaseI.readiness_gate]
