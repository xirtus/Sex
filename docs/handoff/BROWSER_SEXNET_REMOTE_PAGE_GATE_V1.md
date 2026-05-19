# BROWSER_SEXNET_REMOTE_PAGE_GATE_V1

Date: 2026-05-19
Branch: master
Phase K / Task 56

## Gate: browser_sexnet_remote_page

### Description

Daily driver gate proving the browser remote page path through sexnet source=3.

### Gate Logic

**PASS** when ALL of:
- `[browser.sexnet.route.stop_review.pass]` present
- `[browser.sexnet.fetch.request]` mode=consume_last_source3_result ok=1
- `[browser.sexnet.fetch.status]` source=3 http_status=200 body_len=13 ok=1
- `[browser.sexnet.fetch.body]` source=3 bytes=13 bounded=1 ok=1
- `[browser.sexnet.fetch.proof.done]` source=3 fetched=1 status=200 bytes=13 ok=1
- `[browser.sexnet.body.render]` source=3 bytes=13 lines=1 bounded=1 ok=1
- `[browser.sexnet.body.render.proof.done]` source=3 rendered=1 bytes=13 ok=1
- `[browser.sexnet.status.ui]` source=3 status=200 bytes=13 fetched=1 ok=1
- `[browser.sexnet.status.proof.done]` source=3 ok=1
- `[sexnet.netdiag.source3.body.proof.done]` source=3 body_len=13 ok=1
- faults_zero PASS

**SKIP** when:
- Phase K profile not enabled (no SEXNET_PHASE_K_BROWSER_PROOF=1)
- No source=3 HTTP body available (Phase I/J not proven in this boot)
- Default daily boot runs browser stub/static mode

**FAIL** when:
- Browser claims source=3 remote but only static_stub/source=1 markers exist
- Browser uses HAL/source=2 as primary
- Browser gets raw NIC access (impossible — no capability grant)
- Body/render markers contradict source=3 proof
- Framebuffer/display fault markers occur
- Fault scan fails

### Gate Implementation

Added to `scripts/daily_driver_master_gate.sh` alongside existing `sexnet_http_get_source3` and `sexnet_netdiag_source3_primary` gates.

### Relationship to Other Gates

| Gate | Relationship |
|------|-------------|
| `sexnet_http_get_source3` | Prerequisite — source=3 body must be proven |
| `sexnet_netdiag_source3_primary` | Prerequisite — source=3 primary markers must exist |
| `browser_sexnet_remote_page` | Phase K — browser consumes source=3 result |
| `sexnet_browser_cap` | Remains SKIP — no real grant yet |
| `browser_network_grant` | Remains SKIP — deferred |

### Proof Command

```bash
SEXNET_PHASE_I_HTTP_PROOF=1 \
SEXNET_PHASE_K_BROWSER_PROOF=1 \
SEXOS_HAL_TCP_PROBE=0 \
QEMU_NET_BACKEND=user \
QEMU_NET_MODEL=e1000 \
ENABLE_QEMU_USERNET_E1000=1 \
  ./scripts/run_daily_driver_proof.sh /tmp/sexnet_phase_k_browser_route.log

./scripts/daily_driver_master_gate.sh /tmp/sexnet_phase_k_browser_route.log
```
