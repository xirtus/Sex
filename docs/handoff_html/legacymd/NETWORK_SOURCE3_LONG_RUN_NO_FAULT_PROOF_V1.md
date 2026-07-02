# NETWORK_SOURCE3_LONG_RUN_NO_FAULT_PROOF_V1

Date: 2026-05-19
Branch: master
Task: 66 — Phase M network source3 long run no-fault proof

## Goal

Run extended source3 proof profile and prove zero faults over the duration.

## Profile

- Minimum 90s probe window (widened from 30s default)
- e1000 NIC model (QEMU_NET_MODEL=e1000)
- User-mode network backend (QEMU_NET_BACKEND=user)
- Python HTTP peer on host port 18081
- Phase M multi-fetch executes during boot (SEXNET_PHASE_M_RELIABILITY_PROOF=1)
- All existing daily driver gates also evaluated

## Markers

```
[network.source3.long_run.begin] seconds=90 ok=1
[network.source3.long_run.done] seconds=90 faults=0 ok=1
```

## Gate Derivation

The `network_source3_long_run` gate derives from:
- run_daily_driver proof duration >= 90s
- faults_zero gate PASS
- source3 reliability markers present (multi_fetch, descriptor_reuse, retry_policy)
- No fault markers (#PF, #GP, panic, KERNEL PANIC, fault.kill)

## Classification

PASS IMPLEMENTED when:
- Probe window >= 90s
- faults_zero = PASS (0 fault markers)
- source3 reliability markers present
- FINAL PASS

If environment-limited (no HTTP peer): PASS REVIEW ONLY.
