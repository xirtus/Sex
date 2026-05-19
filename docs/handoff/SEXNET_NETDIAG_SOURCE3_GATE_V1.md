# SEXNET_NETDIAG_SOURCE3_GATE_V1

Date: 2026-05-19
Phase: J (Task 51)
Status: IMPLEMENTED

## Gate Name

`sexnet_netdiag_source3_primary`

## Location

`scripts/daily_driver_master_gate.sh`

## Gate Logic

### PASS Conditions (all required)

1. `[sexnet.netdiag.source3.status]` with `source=3 primary=1 ok=1`
2. `[sexnet.netdiag.source3.syscall.proof.done]` with `source=3 primary=1 ok=1`
3. `[sexnet.netdiag.source3.body.proof.done]` with `source=3 ok=1`
4. `[sexnet.phaseI.readiness]` with `source=3 ok=1`
5. `[sexnet.http.status.proof.done]` with `status=200 ok=1`
6. `[sexnet.http.body.proof.done]` with `ok=1`
7. `faults_zero` = PASS

### SKIP Conditions

- Phase I source3 HTTP proof profile is not enabled (default daily boot)
- No HTTP peer/environment available
- Source3 readiness absent (`[sexnet.netdiag.source3.status]` with `primary=0 ok=0`)
- Normal daily boot without explicit Phase I/J profile

### FAIL Conditions

- Source3 primary is claimed (`primary=1`) but body uses source=2/HAL markers
- Body proof claims PASS with zero-length body (`body_len=0`)
- Fault scan fails

### Profile Behavior

| Profile | Expected Gate Result |
|---------|---------------------|
| Default daily boot (no SEXNET_PHASE_I_HTTP_PROOF) | SKIP |
| `SEXNET_PHASE_I_HTTP_PROOF=1` with HTTP peer | PASS |
| `SEXNET_PHASE_I_HTTP_PROOF=1` without HTTP peer | SKIP |

The gate does NOT block default daily runs. SKIP is acceptable and expected in normal daily boot.

## Implementation

```bash
# Gate initialization
gate_sexnet_netdiag_source3_primary="SKIP"

# Gate evaluation (in daily_driver_master_gate.sh)
# PASS: all source3 netdiag markers present + Phase I readiness + HTTP status=200 + zero faults
# SKIP: source3 profile not enabled, Phase I not ready, no HTTP peer
# FAIL: source3 claimed but source=2 body, zero-length body claim, or faults
```

## Gate List Entry

```
"sexnet_netdiag_source3_primary:$gate_sexnet_netdiag_source3_primary"
```

## Doc Marker

```
[sexnet.netdiag.source3.gate.proof.done]
```
