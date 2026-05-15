# DAILY_DRIVER_PROOF_PROFILE_V7 — Handoff

## Goal
Update the daily-driver proof profile from 36 V6 gates to 39 V7 gates,
adding env vars and gate checks for the 3 new lifecycle/editor feature proofs
completed in feature batch V7.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `scripts/run_daily_driver_proof.sh` | +3 env var exports, V6→V7 label | +5 |
| `scripts/daily_driver_master_gate.sh` | +3 gate checks + ALL_GATES, V6→V7 label | +55 |

## New Env Vars (added to proof script)
```
SEXOS_QUIL_EDITOR_KEYBINDINGS_PROOF=1
SEXOS_APP_LIFECYCLE_STATE_PROOF=1
SEXOS_SPINDLE_APP_LIFECYCLE_PROOF=1
```

## New Gates (added to master gate script)
| Gate | Evidence | Proof Marker |
|------|----------|--------------|
| `quil_editor_keybindings` | `[quil.editor.keybind.proof.done] ok=1` | 8 keybinds |
| `app_lifecycle_state` | `[app.lifecycle.proof.done] ok=1` | 7 lifecycle states |
| `spindle_app_lifecycle` | `[spindle.lifecycle.proof.done] ok=1` | 1 lifecycle command |

## Gate Progression
```
V1 (18): keyboard_gui … faults_zero
V2 (22): V1 + app_launch_commands, linen_object_workflow, quil_text_buffer, bell_app_events
V3 (26): V2 + linen_object_persist, quil_text_save, spindle_launch_exec, bell_workflow_events
V4 (30): V3 + app_registry_static, linen_object_schema, quil_text_commands, bell_workflow_detail
V5 (33): V4 + spindle_linen_workflow, spindle_quil_workflow, quil_cursor_nav
V6 (36): V5 + quil_text_selection, quil_text_delete, spindle_editor_v2
V7 (39): V6 + quil_editor_keybindings, app_lifecycle_state, spindle_app_lifecycle
```

## Build + Proof Result
- `entrypoint_build.sh` PASS (10s)
- `run_daily_driver_proof.sh` PASS: **39/39 gates, 0 skipped, 0 faults**

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ✅ All existing V1-V6 gates preserved — zero regression
- ✅ New gates default to SKIP if proof not enabled
- ✅ Fault scan patterns unchanged
- ✅ Linen timing skip fix from V6 stabilization holds (all 39 PASS)

## Known Limitations
- Quil keybindings proof exercises functions directly (not via real scancode dispatch)
- App lifecycle states are synthetic (not runtime-derived)
- Spindle app-state command uses static table (no live query)

## Future Follow-up
- Wire Delete/Ctrl+K/Ctrl+Y scancodes to Quil dispatch
- Runtime lifecycle tracking from PD spawn/terminate events
- Live lifecycle query via PDX opcode
- Quil undo/redo ring implementation (design complete in handoff)
