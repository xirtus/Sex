# HTTP_CLIENT_STUB_SPEC_V1

**Status:** PASS REVIEW ONLY — Docs-only spec.
**Date:** 2026-05-16
**Gates:** 124/124 baseline.

---

## Scope: Bounded HTTP text client stub

Status-only first. Fixed request model. Bounded response. No packets until network route exists. No TLS, CSS, JS, images, media.

---

## Ownership

| Component | Role |
|-----------|------|
| Browser | URL intent, history, tabs |
| Collar | Network grant approval (future) |
| sexnet | Network status/route |
| http_client | Request/response state (future) |
| silk-shell | WebStub surface/session |
| sexdisplay | Pixel rendering only |

---

## Bounded Model (no_std, no heap)

| Field | Max |
|-------|-----|
| url_len | 256 |
| host_len | 128 |
| path_len | 256 |
| response_bytes | 4096 |

Status enum: no_grant, no_route, request_ready, sent, response_bounded, blocked_no_tcp

---

## Phase Ladder

| Phase | What |
|-------|------|
| 0 | This spec |
| 1 | Status stub, fetched=0 |
| 2 | Fixed request builder, no send |
| 3 | sexnet route handshake, no packets |
| 4 | Static-IP HTTP GET after network route |
| 5 | Response → HTML subset parser |
| 6 | DNS/TLS later |

---

## STOP FIRST Boundaries

- No SLOT_NET grant without Collar approval
- No network/fetch claims before route exists
- No heap/std/POSIX sockets
- No unbounded buffers
- No TLS/crypto
- No sexdisplay changes

---

## Next: HTTP_CLIENT_STATUS_STUB_V1

## Commit
```bash
git add docs/handoff/HTTP_CLIENT_STUB_SPEC_V1.md
git commit -m "docs(net): HTTP client stub spec V1"
```
