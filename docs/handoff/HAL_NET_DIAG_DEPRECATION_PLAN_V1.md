# HAL_NET_DIAG_DEPRECATION_PLAN_V1

Date: 2026-05-19
Branch: master
Phase: L (HAL NET_DIAG freeze/legacy, source3 primary)

## Purpose

Deprecate HAL NET_DIAG/source=2 as the primary network diagnostic truth,
reclassifying it as legacy/fallback while source=3 remains primary for
HTTP/browser network truth. No HAL code is deleted in Phase L.

## Review Questions Answered

### 1. Where are HAL NET_DIAG/source=2 markers emitted?
- `kernel/src/hal/pci.rs` lines 3438-3440: HAL HTTP response status/bytes/source
  stored in `NET_DIAG_HTTP_STATUS`, `NET_DIAG_HTTP_BYTES`, `NET_DIAG_HTTP_SOURCE`
  and body in `NET_DIAG_HTTP_BODY`.
  Source is set to 2 (real) or 1 (mock) via `NET_DIAG_HTTP_SOURCE.store(2)`.
- `servers/sexnet/src/main.rs` line 529: `[sexnet.dynamic_body.set] len=N source=2 ok=1`
  when sexnet reads HAL NET_DIAG body via `sys_net_diag()`.
- `servers/sexnet/src/main.rs` lines 481-510: `sys_net_diag(0)` returns HAL status,
  `sys_net_diag(1)` returns body length, `sys_net_diag(2+ci)` returns packed body bytes.

### 2. Which HAL diagnostics still run by default?
- The HAL TCP probe (kernel/src/hal/pci.rs ~line 2732) runs by default unless
  `SEXOS_HAL_TCP_PROBE=0` is set at compile time.
- HAL DNS diagnostic (Phase F) uses source=2 for A-record resolution.
- HAL HTTP diagnostic is always present as a status store but is only populated
  when the HAL TCP probe engages.

### 3. Which HAL diagnostics already have freeze gates?
- `[hal.tcp.probe.gate] enabled=0 reason=SEXOS_HAL_TCP_PROBE=0 ok=1` (pci.rs line 3504)
  fires when `SEXOS_HAL_TCP_PROBE=0` is set.
- `scripts/run_daily_driver_proof.sh` line 264 defaults `SEXOS_HAL_TCP_PROBE=0`
  when Phase I explicit profile is active.
- No other HAL diagnostic has an explicit freeze gate yet.

### 4. Which source2 proofs are still useful as legacy/fallback?
- DNS A-record resolution and cache proof (Phase F): review-only via source=2.
  Useful for future source=3 DNS migration.
- HAL HTTP body capture: useful for rollback verification and diagnostic cross-check.
- HAL NIC enumeration/e1000 probe: useful for hardware bringup and fallback NIC init.

### 5. Does any daily gate still treat source2 as primary?
- `net_real_http_body_prefix` gate (~line 2988-3005) accepts source=2 markers
  (`sexnet.dynamic_body.set.*len=64.*source=2.*ok=1`) for real(2)->sexnet path.
  This gate is a legacy pass-through and does not assert primacy.
- No gate currently treats source=2 as *primary* for HTTP or browser truth.
  The new source3 gates (sexnet_http_get_source3, sexnet_netdiag_source3_primary,
  browser_sexnet_remote_page) all require source=3 markers.

### 6. Does source3 now cover HTTP/browser route?
- YES: Phase I (HTTP GET over TCP source=3), Phase J (netdiag source3 primary),
  Phase K (browser remote page through source3) are all PASS IMPLEMENTED.
- source=3 covers: HTTP GET TX/RX/status/body, browser fetch/body/render/status UI.

### 7. What remains not source3?
- DNS resolution: still source=2 only (Phase F, review-only).
- Real PDX browser→sexnet live fetch route: marker-only in Phase K.
  Live fetch route is deferred to Phase L+ but does not block this freeze gate.
- Real hardware audit: deferred to Phase N.
- TLS, JS, HTML engine: deferred beyond current phases.

### 8. Can Phase L freeze source2 primary behavior without kernel/ABI edits?
- YES. The freeze is achieved through:
  a) Documentation: this plan + freeze gate + handoff docs classify source2 as legacy.
  b) Gate scripts: new hal_net_diag_freeze and network_source3_primary gates enforce
     that source2 cannot be counted as primary when source3 is present.
  c) Profile/env: `SEXOS_HAL_TCP_PROBE=0` disables HAL TCP probe in source3 profile.
  d) Marker addition: `[hal.netdiag.freeze]` marker in sexnet main.rs (optional,
     safe additive marker).
- No kernel/HAL code is deleted.
- No syscall ABI is changed.
- No sex-pdx ABI is changed.
- HAL code remains compilable and reachable for rollback.

### 9. STOP FIRST boundaries
- DO NOT delete HAL NET_DIAG runtime code (kernel/src/hal/pci.rs).
- DO NOT change sys_net_diag syscall ABI.
- DO NOT change sex-pdx ABI.
- DO NOT change NIC ownership model.
- DO NOT grant browser raw NIC access.
- DO NOT migrate DNS to source=3.
- DO NOT change browser route behavior beyond safety markers.
- DO NOT change kernel HAL runtime behavior.

## Deprecation Policy

- source=3 PRIMARY for HTTP/browser network truth.
- source=2 HAL NET_DIAG LEGACY/FALLBACK only.
- source=2 may still provide legacy diagnostics and DNS review-only evidence.
- No source=2 marker may be counted as source=3.
- No HAL deletion in Phase L.
- No DNS source=3 claim.

## Conclusion

**PASS REVIEW ONLY** — HAL NET_DIAG can be deprecated by docs/gates/profile
markers without unsafe runtime changes. No kernel/HAL/ABI edits required.
The freeze is enforced through gate scripts, environment variables, and
documentation classification. Actual HAL retirement/deletion is deferred
to post-Phase M/N after reliability/hardware audit.

## Doc Marker

[hal.netdiag.deprecation.plan.pass]
