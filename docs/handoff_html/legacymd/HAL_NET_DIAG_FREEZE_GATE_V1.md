# HAL_NET_DIAG_FREEZE_GATE_V1

Date: 2026-05-19
Branch: master
Phase: L, Task 58

## Gate Name

`hal_net_diag_freeze`

## Purpose

Prove that HAL NET_DIAG/source=2 is frozen as primary diagnostic truth and
cannot override source=3 when the explicit source3 profile is active.

## PASS Conditions

Gate passes when ALL of the following are true:
1. source=3 primary markers are present (sexnet_http_get_source3 PASS,
   sexnet_netdiag_source3_primary PASS, browser_sexnet_remote_page PASS).
2. HAL TCP probe freeze marker is present: `[hal.tcp.probe.gate] enabled=0 ... ok=1`
   OR HAL/source2 markers are classified as legacy/fallback only.
3. No source2/HAL markers claim primary when source3 is present.
4. Faults zero.
5. sexnet source3 body is nonzero through explicit profile.

## SKIP Conditions

Gate SKIPs when:
- Explicit source3 profile is not enabled (SEXNET_PHASE_I_HTTP_PROOF != 1).
- HTTP peer absent (TCP handshake cannot complete).
- Daily default mode with no explicit source3 profile.

## FAIL Conditions

Gate FAILs when:
- source2 is marked primary while source3 proof is present.
- HAL TCP probe runs during explicit source3 primary proof and competes with sexnet TCP.
- source3 gates pass but HAL source2 overwrites primary status/body.
- source2 body/status is accepted as source3.
- Fault scan fails.

## Implementation

### Gate Logic (in scripts/daily_driver_master_gate.sh)

```
# Gate: hal_net_diag_freeze — Phase L HAL NET_DIAG frozen as legacy
# PASS: source3 primary gates pass + HAL TCP probe disabled + source2 legacy-only
# SKIP: explicit profile not active
# FAIL: source2 claims primary while source3 is present
```

### Runtime Marker (in servers/sexnet/src/main.rs)

Added during Phase L:
```
[hal.netdiag.freeze] source2=legacy source3=primary ok=1
```

This marker fires when source3 Phase I readiness is proven and HAL TCP probe
is disabled, confirming that HAL NET_DIAG is safely frozen as legacy/fallback.

### Profile Integration

`scripts/run_daily_driver_proof.sh` already sets `SEXOS_HAL_TCP_PROBE=0` when
`SEXNET_PHASE_I_HTTP_PROOF=1`, which triggers the `[hal.tcp.probe.gate] enabled=0`
marker in the kernel HAL.

## Edge Cases

- If source3 readiness fails but HAL also disabled: gate SKIPs honestly.
- If HAL TCP probe gate fires (enabled=0) but source3 absent: gate SKIPs.
- If HAL TCP probe runs despite SEXOS_HAL_TCP_PROBE=0: this is a compile-time
  gate failure and gate FAILs.

## Doc Marker

[hal.netdiag.freeze] source2=legacy source3=primary ok=1
