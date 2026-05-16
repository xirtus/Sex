# BROWSER_HTTP_TEXT_FETCH_PLAN_V1

**Status:** PASS REVIEW ONLY — Docs-only plan. No implementation.
**Date:** 2026-05-16
**Gates:** 119/119 baseline.

---

## Plan Summary

Future: Browser fetches plain HTTP text over a Collar-approved network path.
First fetch is bounded, plain-text only, no TLS, no images, no CSS/JS.

---

## Ownership Table

| Component | Role |
|-----------|------|
| Browser/WebStub | URL intent, history, tabs, render state |
| sexnet | Network I/O (future) |
| HTTP client | Request/response parsing (future) |
| Collar | Network grant approval |
| Mesh | Network route visualization |
| silk-shell | Surface/session/focus policy |
| sexdisplay | Pixel rendering only |

---

## Capability Contract

- Browser requires explicit network grant from Collar
- URL intent alone does NOT grant fetch authority
- Link click is marker-only until grant + network path exist
- Fetched content: bounded response size, text/plain only
- No ambient network authority

---

## Phase Ladder

| Phase | What |
|-------|------|
| 0 | This plan |
| 1 | sexnet route/status audit |
| 2 | Network capability stub (no packets) |
| 3 | DNS plan or static IP HTTP plan |
| 4 | HTTP GET text fetch (local/test endpoint) |
| 5 | Bounded response buffer |
| 6 | Feed response to HTML subset parser |
| 7 | TLS plan |
| 8 | Images/media |

---

## Proof Gates

- network_grant=0 until Collar approves
- fetched=0 until real response bytes exist
- response_len bounded
- css=0, js=0 until later phases
- 0 faults, no unbounded payload logs

---

## Next Prompt: SEXNET_BROWSER_ROUTE_AUDIT_V1

## Commit
```bash
git add docs/handoff/BROWSER_HTTP_TEXT_FETCH_PLAN_V1.md
git commit -m "docs(browser): HTTP text fetch plan V1"
```
