# NETWORK_SOURCE3_PRIMARY_GATE_V1

Date: 2026-05-19
Branch: master
Phase: L, Task 60

## Gate Name

`network_source3_primary`

## Purpose

Prove that source=3 is the sole primary network diagnostic truth
and that Phase I+J+K source3 proofs are all passing while HAL/source=2
is correctly classified as legacy/fallback.

## PASS Conditions

Gate passes when ALL of:
1. Phase I source3 HTTP GET passes (sexnet_http_get_source3 = PASS).
2. Phase J source3 netdiag primary passes (sexnet_netdiag_source3_primary = PASS).
3. Phase K browser remote page through source3 passes (browser_sexnet_remote_page = PASS).
4. HAL/source2 is classified legacy/fallback (hal_net_diag_freeze = PASS).
5. source3 body/status are nonzero and status=200.
6. Faults zero.

## SKIP Conditions

Gate SKIPs when:
- Explicit source3 profile not enabled.
- HTTP peer absent (TCP handshake cannot complete).
- Browser proof profile not enabled.
- Any prerequisite gate is SKIP (no partial PASS).

## FAIL Conditions

Gate FAILs when:
- source2/HAL is counted as primary while source3 is present.
- Browser remote page uses static/source1/source2 while claiming source3.
- source3 body/status absent while gate claims primary.
- Fault scan fails.
- Any prerequisite gate is FAIL.

## Implementation

### Gate Logic (in scripts/daily_driver_master_gate.sh)

The gate derives its result from prerequisite gates rather than duplicating
marker checks:
- Requires sexnet_http_get_source3 == PASS
- Requires sexnet_netdiag_source3_primary == PASS
- Requires browser_sexnet_remote_page == PASS
- Requires hal_net_diag_freeze == PASS
- Requires faults_zero == PASS

### Source Ownership Classification (enforced)

| Source | Role | Gates |
|--------|------|-------|
| source=3 | PRIMARY | sexnet_http_get_source3, sexnet_netdiag_source3_primary, browser_sexnet_remote_page |
| source=2 | LEGACY/FALLBACK | hal_net_diag_freeze, hal.tcp.probe.gate enabled=0 |
| source=1 | MOCK/OFFLINE | Built-in static text |

## Integration

This gate is the Phase L culmination gate. When it passes, Phase L is
complete: the network stack has a single primary source (source=3) with
HAL diagnostics safely frozen as legacy/fallback.

## What This Gate Does NOT Prove

- source=3 DNS (still HAL/source=2 only)
- Real hardware NIC audit (Phase N)
- Multi-fetch reliability (Phase M)
- Stress testing (Phase M)
- Real PDX browser→sexnet live fetch (Phase L+, marker-only in Phase K)
- HAL code deletion (deferred to post-Phase M/N)

## Doc Marker

[network.source3.primary.gate] ok=1
