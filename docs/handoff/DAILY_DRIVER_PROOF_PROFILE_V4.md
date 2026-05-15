# DAILY_DRIVER_PROOF_PROFILE_V4 — Handoff

## Goal
Update the daily-driver proof profile from 26 V3 gates to 30 V4 gates,
adding env vars and gate checks for the 4 new registry/schema feature proofs
completed in feature batch V4.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `scripts/run_daily_driver_proof.sh` | +4 env var exports, V3→V4 label | +6 |
| `scripts/daily_driver_master_gate.sh` | +4 gate checks + ALL_GATES, V3→V4 label | +73 |

## New Env Vars (added to proof script)
```
SEXOS_APP_REGISTRY_STATIC_V2_PROOF=1
SEXOS_LINEN_OBJECT_SCHEMA_PROOF=1
SEXOS_QUIL_TEXT_COMMANDS_PROOF=1
SEXOS_BELL_WORKFLOW_DETAIL_PROOF=1
```

## New Gates (added to master gate script)
| Gate | Evidence | Proof Marker |
|------|----------|--------------|
| `app_registry_static` | `[app.registry.proof.done] ok=1` | 8 registry rows |
| `linen_object_schema` | `[linen.schema.proof.done] ok=1` | 3 kinds, 4 statuses |
| `quil_text_commands` | `[quil.text.command.proof.done] ok=1` | 4 commands |
| `bell_workflow_detail` | `[bell.workflow.detail.proof.done] ok=1` | 4 detail markers |

## Gate Progression
```
V1 (18): keyboard_gui … silkbar_phase5_pixels, faults_zero
V2 (22): V1 + app_launch_commands, linen_object_workflow, quil_text_buffer, bell_app_events
V3 (26): V2 + linen_object_persist, quil_text_save, spindle_launch_exec, bell_workflow_events
V4 (30): V3 + app_registry_static, linen_object_schema, quil_text_commands, bell_workflow_detail
```

## Build + Proof Result
- `entrypoint_build.sh` PASS (9s)
- `run_daily_driver_proof.sh` PASS: 30/30 gates, 0 skipped, 0 faults

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ✅ All existing V1-V3 gates preserved — zero regression
- ✅ New gates default to SKIP if proof not enabled (backward compatible)
- ✅ Fault scan patterns unchanged

## Known Limitations
- All 4 new proofs are synthetic marker-only (schema taxonomy, static registry)
- App registry is static compile-time table (no live query)
- Linen schema not enforced in runtime code
- Bell detail not backed by real queue inspection

## Future Follow-up
- Live app registry query via PDX opcode
- Runtime schema enforcement (status field on LinenObject)
- User-facing keybindings for Quil editor commands
- Bell detail readback from server queue
