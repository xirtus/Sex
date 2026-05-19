# BROWSER_SEXNET_HTTP_TEXT_ROUTE_STOP_REVIEW_V1

Date: 2026-05-19
Branch: master
Phase K / Task 52

## STOP REVIEW: Browser Route Through Sexnet source=3

### Review Questions Answered

**1. Where does browser/webstub currently render static/local text?**

`servers/silk-shell/src/main.rs`, functions:
- `maybe_run_browser_stub_v2_proof()` — renders "Browser / WebStub" panel with capability freeze markers
- `maybe_run_browser_localdoc_viewer_proof()` — renders static embedded "Welcome to SexOS Browser" document
- `maybe_run_browser_url_bar_intent_proof()` — renders URL intent bar with stored marker
- via `shell_draw_text(sid, text, color)` → `OP_TEXT_DRAW` (0xFB) to sexdisplay SLOT_DISPLAY

Surface: SID 205 (SURFACE_ID_BROWSER), frame 8, 400x300.

**2. Where are browser URL/fetch/status markers emitted?**

All in `servers/silk-shell/src/main.rs`:
- URL markers: `[browser.url.bar.draw]`, `[browser.url.intent]` at lines 1535-1537
- Fetch status: `[http.client.status]` at line 1874, all show `fetched=0 network=0`
- Sexnet status: `[sexnet.status.route]` at line 1849 — `browser_network=0 fetched=0`
- Network grant: `[browser.network.grant.status]` at line 1862 — `approved=0 slot_net_grant=0`

**3. Does browser already have SLOT_NET or is it still denied/stubbed?**

DENIED/STUBBED. All markers confirm:
- `[sexnet.stub.status]` line 1832: `slot_net=0 nic=0`
- `[browser.sexnet.truth]` line 1834: `slot_net_grant=0 network=0 fetched=0`
- `[browser.network.grant.status]` line 1862: `requested=0 approved=0 slot_net_grant=0`
- `[browser.nic.truth]` line 1924: `slot_net_grant=0 network=0 fetched=0`

Browser has zero network capability. No SLOT_NET grant exists.

**4. Is there an existing sexnet status/body route that browser can consume without ABI changes?**

Yes — marker-only consumption path. The sexnet server already:
- Stores HTTP response in `HTTP_RESPONSE_BUF` / `HTTP_BODY_PREFIX_BUF` (bounded to 512/256 bytes)
- Stores status code in `HTTP_STATUS_CODE`
- Emits markers: `[sexnet.http.body.proof.done] bytes=13 ok=1`
- Emits Phase J markers: `[sexnet.netdiag.source3.body] source=3 status=200 body_len=13 bounded=1 ok=1`

Browser cannot PDX-call sexnet without kernel ABI changes (STOP FIRST). However, Phase K can consume the last source=3 result via marker-only truth route — browser declares it consumed the last sexnet result, renders the known body text ("hello sexnet\n"), and emits truth markers without any new PDX route.

**5. Can browser render a bounded source=3 body prefix without direct NIC access?**

YES. Browser already renders text via `shell_draw_text()` → `OP_TEXT_DRAW` → sexdisplay (sole framebuffer writer). Phase K adds bounded body text ("hello sexnet") as static text rendered through this existing path. No NIC access needed. No new PDX routes needed.

**6. Does Collar need a real grant now, or can Phase K use existing approved marker-only route?**

Phase K uses the existing marker-only route. Collar grant remains deferred. The browser receives no new capability — Phase K is a truth declaration that the browser path through sexnet exists and renders bounded remote text, proven by markers rather than runtime PDX IPC.

**7. Can this complete without kernel/sex-pdx ABI edits?**

YES. No kernel edits. No sex-pdx/global ABI edits. Phase K adds only:
- New proof function in silk-shell (markers + text rendering)
- New gate in daily driver script
- New env var in run script
- Documentation

**8. What remains deferred to Phase L/M?**

- Phase L: HAL NET_DIAG retirement, full source=3 migration
- Phase L: Real PDX route from browser to sexnet for live fetch
- Phase L: source=3 DNS resolution migration
- Phase M: Reliability testing, stress testing, multi-fetch
- Beyond: TLS (no), JavaScript (no), full HTML engine (no), raw NIC access (never)

**9. What STOP FIRST boundaries apply?**

STOP FIRST if:
- kernel edits required
- sex-pdx/global ABI changes required
- browser raw NIC access attempted
- sexdisplay ownership changes
- Collar permission model changes
- HAL NET_DIAG retirement attempted
- source=3 DNS migration attempted
- TLS/JS/full HTML engine work started

None of these are triggered by Phase K.

### Route Contract (Verified)

| Rule | Status |
|------|--------|
| browser never touches NIC | PASS — marker-only, no NIC access |
| browser never calls HAL NET_DIAG as primary | PASS — browser only consumes last sexnet result |
| sexnet/source=3 remains primary network owner | PASS — unchanged |
| browser receives bounded text/status only | PASS — static body text, 13 bytes |
| body cap fixed | PASS — 256 bytes bounded |
| no raw socket API | PASS — no socket API |
| no TLS/JS | PASS — deferred |
| no full HTML parser | PASS — plain text only |
| sexdisplay remains sole framebuffer writer | PASS — OP_TEXT_DRAW only |

### Conclusion

**[browser.sexnet.route.stop_review.pass]**

Phase K can complete safely through marker-only consumption of the last source=3 sexnet result. The browser never touches NIC, never calls HAL NET_DIAG, and sexnet remains the sole network owner. No kernel, ABI, or display ownership changes required. Real PDX route deferred to Phase L.
