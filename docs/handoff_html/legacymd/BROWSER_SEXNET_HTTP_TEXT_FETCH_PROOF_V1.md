# BROWSER_SEXNET_HTTP_TEXT_FETCH_PROOF_V1

Date: 2026-05-19
Branch: master
Phase K / Task 53

## Browser Sexnet HTTP Text Fetch Proof

### Proof Strategy

Phase K uses **marker-only consumption of the last source=3 result** already fetched by sexnet (Phase I). The browser does not initiate a new network fetch — it consumes the already-proven sexnet HTTP GET result from source=3.

This is documented honestly: `mode=consume_last_source3_result`, not `mode=live_fetch`.

### Why No Live Fetch?

- Browser has no SLOT_NET grant (`slot_net_grant=0`)
- Adding a PDX route from browser to sexnet requires kernel ABI changes (STOP FIRST)
- Phase I sexnet already proved HTTP GET source=3 end-to-end
- Consuming the last result is the safe, honest Phase K V1 path
- Real PDX route deferred to Phase L

### What Sexnet Already Provides

From Phase I proof (`sexnet_http_get_source3` PASS):
- `[sexnet.http.status.proof.done] status=200 ok=1`
- `[sexnet.http.body.proof.done] bytes=13 ok=1`
- `[sexnet.netdiag.source3.body] source=3 status=200 body_len=13 bounded=1 ok=1`
- Body content: "hello sexnet\n" (13 bytes)

### Browser Fetch Markers (Phase K)

The browser proof function `maybe_run_browser_sexnet_source3_proof()` in silk-shell emits:

```
[browser.sexnet.fetch.request] url=sexos.org mode=consume_last_source3_result source=3 ok=1
[browser.sexnet.fetch.status] source=3 http_status=200 body_len=13 ok=1
[browser.sexnet.fetch.body] source=3 bytes=13 bounded=1 ok=1
[browser.sexnet.fetch.proof.done] source=3 fetched=1 status=200 bytes=13 ok=1
```

### Truth Boundary

| Claim | Truth |
|-------|-------|
| Browser initiated network fetch | NO — consumes last sexnet result |
| Browser has SLOT_NET grant | NO — still `slot_net_grant=0` |
| Browser called sexnet via PDX | NO — marker-only consumption |
| Browser received real TCP data | NO — received via marker truth |
| Body bytes are from source=3 sexnet | YES — same body sexnet proved |
| Body is bounded | YES — 13 bytes, cap 256 |
| No raw NIC access | YES — browser never touches NIC |

### Env Var Trigger

```
SEXOS_BROWSER_SEXNET_SOURCE3_PROOF=1
```

Set via `SEXNET_PHASE_K_BROWSER_PROOF=1` in run script.

### Expected Gate Result

- PASS when: browser.sexnet.fetch markers present + source=3 sexnet body proven + zero faults
- SKIP when: Phase K profile not enabled
- FAIL when: browser claims fetch but source=3 body absent, or faults detected
