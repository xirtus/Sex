# BROWSER_SEXNET_STATUS_UI_PROOF_V1

Date: 2026-05-19
Branch: master
Phase K / Task 55

## Browser Sexnet Status UI Proof

### Goal

Browser UI/status area on SID 205 shows source=3 remote fetch state, distinguishing:
- source=3 sexnet remote (Phase K)
- source=1 static/local stub (Phase 1)
- unavailable/skipped

### Status UI Elements Rendered

On browser surface (SID 205), the status area shows:

```
Browser / WebStub  —  source=3 REMOTE
────────────────────────────────────────
source=3    HTTP/1.1 200 OK    bytes=13
fetched=1   sexnet route       bounded=1
────────────────────────────────────────
body: hello sexnet
────────────────────────────────────────
network=0 slot_net_grant=0 no_raw_nic=1
[ capability freeze: no NIC, no HAL ]
```

### Markers

```
[browser.sexnet.status.ui] source=3 status=200 bytes=13 fetched=1 ok=1
[browser.sexnet.status.label] text=source3_sexnet_remote ok=1
[browser.sexnet.status.label] text=http_200_ok ok=1
[browser.sexnet.status.label] text=bytes_13_bounded ok=1
[browser.sexnet.status.proof.done] source=3 ok=1
```

### Status UI State Machine

| Source | Network | Fetched | Body | Label |
|--------|---------|---------|------|-------|
| source=3 | 0 (no NIC grant) | 1 | 13 bytes | sexnet_remote |
| source=1 | 0 | 0 | 0 | static_local_stub |
| none | 0 | 0 | 0 | unavailable_skipped |

### Design Rules

| Rule | Status |
|------|--------|
| No visual redesign | PASS — existing surface, new text lines |
| Sexdisplay sole FB writer | PASS — OP_TEXT_DRAW only |
| Distinguishes static vs remote | PASS — source labels |
| Distinguishes source=3 vs source=2 | PASS — source=3 label only, HAL not primary |
| Body cap shown | PASS — bounded=1 marker |

### Truth Invariants

- Browser never touches NIC: `no_raw_nic=1`
- Browser never calls HAL NET_DIAG primary: confirmed
- Sexnet remains sole network owner: confirmed
- All buffers bounded: confirmed (13 bytes)
