# GATE_0_2_PERSISTENT_RUNBOOK

Status: ACTIVE (must be rerun until GREEN_0_2)
Owner: Input stabilization (QEMUX USB/trackpad + SexOS input flow)

## Canonical Gate Script

Use this script as the single checkpoint gate for the 0.2 desktop/input milestone:

- `scripts/gate_0_2.sh`

Do not replace with ad-hoc marker checks. Keep rerunning this gate after each input-related fix.

## Current Blockers

1. QEMUX USB/trackpad injection path is not consistently producing nonzero pointer movement in the SexOS flow.
2. Keyboard path remains incomplete in the same probe lane when PS/2 IRQ1/INPUT_RING markers are missing.
3. Environment/runtime backend can fail interactive/QMP capture in some sessions; this must be treated as a run condition issue, not a false milestone pass.

## Required Workflow Until Green

1. Run `./scripts/gate_0_2.sh`
2. Read `docs/handoff/GATE_0_2_LAST_RUN.md`
3. Fix only the first missing marker owner
4. Re-run `./scripts/gate_0_2.sh`
5. Repeat until `FINAL_SCORE: GREEN_0_2`

## Rule

No new GUI/input/window feature expansion until `scripts/gate_0_2.sh` is GREEN_0_2.

## Last Known Artifacts

- Gate script: `scripts/gate_0_2.sh`
- Last run record: `docs/handoff/GATE_0_2_LAST_RUN.md`
