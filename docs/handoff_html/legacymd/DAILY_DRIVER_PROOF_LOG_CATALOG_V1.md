# DAILY_DRIVER_PROOF_LOG_CATALOG_V1

## Goal
Provide a compact naming convention for overnight proof logs to reduce ambiguity.

## Suggested Log Names
- `/tmp/sexos_<mission_slug>.log`
- Examples:
  - `/tmp/sexos_app_registry_launch_intent_v1.log`
  - `/tmp/sexos_v4_m3_m5.log`

## Required Capture Metadata per Run
1. commit SHA
2. mission id(s)
3. env vars enabled
4. final gate result (`PASS/FAIL`, gate count, faults)

## Suggested Summary Block
- `mission:`
- `sha:`
- `env:`
- `gate:`
- `faults:`
- `notes:`

## Retention
- Keep most recent mission log + final nightly log.
- Prune intermediate logs after handoff docs capture final evidence.
