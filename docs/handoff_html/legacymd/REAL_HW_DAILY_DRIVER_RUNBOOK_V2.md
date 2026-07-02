# REAL_HW_DAILY_DRIVER_RUNBOOK_V2

## Goal
Operator-grade real-hardware checklist for keyboard-first daily-driver proof.

## Preflight
1. Confirm latest ISO rebuilt via `./scripts/entrypoint_build.sh`.
2. Record git SHA and branch.
3. Prepare serial capture target path.
4. Verify known marker grep list is available.

## Capture Checklist
1. Boot real hardware with current ISO.
2. Capture serial/console logs from boot to desktop idle.
3. Execute keyboard interaction sequence:
   - open palette
   - navigate rows
   - execute core app rows
   - verify Bell/Atlas/Linen interactions
4. Save evidence log and short operator notes.

## Required Marker Families
- `shell.palette.*`
- `launcher.*`
- `spindle.*`
- `linen.*`
- `bell.*`
- `atlas.*`
- `silkbar.*`

## Verdict Language
- `PASS`: required interactions and marker families present; no critical faults.
- `PARTIAL PASS`: core boot + some interactions verified; blocker documented.
- `FAIL`: key interaction path broken or unrecoverable faults.

## Blocker Template
- Title:
- Repro steps:
- Expected markers:
- Actual markers:
- Fault lines (if any):
- Last known good SHA:
- STOP FIRST needed? (`yes/no` + reason)

## Notes
- For input/pointer claims, real hardware/GTK evidence is authoritative.
- Do not overclaim runtime success from build-only results.
