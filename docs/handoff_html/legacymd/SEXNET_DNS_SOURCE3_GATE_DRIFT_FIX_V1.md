# SEXNET_DNS_SOURCE3_GATE_DRIFT_FIX_V1

## Problem

`sexnet_dns_source3_proof_v1` FAILs in default daily boot with:
> source2 DNS markers present in source3 proof lane

### Root Cause

The gate variable `dns_s3_source2_used_markers` (line 2797) uses the regex:
```
sexnet\.dns\.(query\.build|query\.tx|response\.parse|cache\.)
```

This matches **legacy kernel DNS cache markers** emitted unconditionally at boot:
- `[sexnet.dns.cache.init] cap=N`   (kernel/src/hal/pci.rs:2240)
- `[sexnet.dns.cache.insert] idx=N …`  (kernel/src/hal/pci.rs:2670)

In default daily mode where `SEXNET_DNS_SOURCE3_PROOF` is **not** enabled, source3
code never runs.  However the legacy kernel cache markers are always present,
so `dns_s3_source2_used_markers >= 1` is always true.

The proof gate (line 2879, now 2879) catches this before reaching the final
SKIP clause, producing an incorrect FAIL.

### Fix

1. Added `dns_s3_active` variable (lines 2799-2808) that detects whether source3
   code was actually exercised.  It checks for any source3 marker:
   - query build (ok=1 or ok=0 → code ran)
   - UDP TX (ok=1, ok=0, or tx_dd=0 → code ran)
   - RX parse (ok=1)
   - answer (ok=1)
   - cache insert (ok=1)
   - RX timeout (no_response_env_blocked → code ran)
   - UDP TX skip (no TX owner → code ran)

2. Changed proof gate condition at line 2879 from:
   ```bash
   elif [ "$dns_s3_source2_used_markers" -ge 1 ]; then
   ```
   to:
   ```bash
   elif [ "$dns_s3_active" -eq 1 ] && [ "$dns_s3_source2_used_markers" -ge 1 ]; then
   ```

3. Backup created at:
   `scripts/daily_driver_master_gate.sh.bak_source3_gate_drift_v1`

### Behavior Matrix

| source3 active? | source2 markers? | Result | Detail |
|---|---|---|---|
| No  | Yes (expected) | SKIP   | "source3 DNS proof lane not exercised" |
| No  | No             | SKIP   | "source3 DNS proof lane not exercised" |
| Yes | No             | → PASS or next check | Passes to next elif chain |
| Yes | Yes            | FAIL   | "source2 DNS markers present in source3 proof lane" |

### Regressions Prevented

- Source3-active + source2-contamination → still FAILs (real regression caught)
- Source3-active + all-clean → still can PASS (full proof path preserved)
- Source3-active + env-blocked → still SKIPs (env-blocked lane preserved)
- Other FAIL conditions (malformed, tx_dd=0, browser-no-evidence, faults) unchanged

### Files Touched

- `scripts/daily_driver_master_gate.sh` — gate logic only; no kernel/ABI/sex-pdx changes
- `docs/handoff/SEXNET_DNS_SOURCE3_GATE_DRIFT_FIX_V1.md` — this doc

### Not Touched

- No kernel/ABI/sex-pdx changes
- No Silk files
- No network source/protocol changes
- No source3 profile detection changes (gate is purely log-marker based)
