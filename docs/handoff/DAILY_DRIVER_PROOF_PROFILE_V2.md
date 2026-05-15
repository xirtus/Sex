# DAILY_DRIVER_PROOF_PROFILE_V2 — Handoff

## Goal
Update the daily-driver proof profile from 18 V1 gates to 22 V2 gates,
adding env vars and gate checks for the 4 new feature proofs completed
in batch 2.6.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `scripts/run_daily_driver_proof.sh` | +4 env var exports | +6 |
| `scripts/daily_driver_master_gate.sh` | +4 gate checks + ALL_GATES integration | +73 |

## New Env Vars (added to proof script)
```
SEXOS_APP_LAUNCH_COMMANDS_PROOF=1
SEXOS_LINEN_OBJECT_WORKFLOW_PROOF=1
SEXOS_QUIL_TEXT_BUFFER_PROOF=1
SEXOS_BELL_APP_EVENT_INTEGRATION_PROOF=1
```

## New Gates (added to master gate script)

| Gate | Evidence | Proof Marker |
|------|----------|--------------|
| `app_launch_commands` | `[spindle.app.proof.done] ok=1` | 19 rows |
| `linen_object_workflow` | `[linen.object.workflow.proof.done] ok=1` | 3 creates, 3 searches |
| `quil_text_buffer` | `[quil.text.buffer.proof.done] ok=1` | 7 recv events |
| `bell_app_events` | `[bell.app.integration.proof.done] ok=1` | 4 events |

## Gate Progression
```
V1 (18 gates): keyboard_gui, command_palette, spindle_daily, spindle_bridges,
               linen_nonblocking, linen_detail, quil_keyboard, bell_events,
               atlas_theme, collar_nav, mesh_nav, silkbar_status,
               launcher_multi_exec, palette_linen_available, quil_status_ready,
               silkbar_phase3_status, silkbar_phase5_pixels, faults_zero

V2 (22 gates): V1 + app_launch_commands, linen_object_workflow,
               quil_text_buffer, bell_app_events
```

## Build + Proof Result
- `entrypoint_build.sh` PASS (8s)
- `run_daily_driver_proof.sh` PASS: 22/22 gates, 0 skipped, 0 faults

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ✅ All existing V1 gates preserved — zero regression
- ✅ New gates default to SKIP if proof not enabled (backward compatible)
- ✅ Fault scan patterns unchanged

## Known Limitations
- All 4 new proofs are synthetic (boot-time auto-execute, not user-triggered)
- Not yet wired to real cross-PD app lifecycle events
- Linen workflow uses local session only (no RamFS/DiskFS persistence)

## Future Follow-up
- Wire Bell events to real app open/close hooks
- Persistent Linen tags via SexFiles
- Quil buffer save to DiskFS
- Spindle live app registry via silk-shell PDX query
