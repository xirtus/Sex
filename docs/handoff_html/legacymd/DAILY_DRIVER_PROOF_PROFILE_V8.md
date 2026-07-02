# DAILY_DRIVER_PROOF_PROFILE_V8 — Handoff

## Goal
Update the daily-driver proof profile from 39 V7 gates to 43 V8 gates,
adding env vars and gate checks for the 4 new undo/lifecycle feature proofs.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `scripts/run_daily_driver_proof.sh` | +4 env var exports, V7→V8 label | +6 |
| `scripts/daily_driver_master_gate.sh` | +4 gate checks + ALL_GATES, V7→V8 label | +70 |

## New Env Vars
```
SEXOS_QUIL_UNDO_REDO_PROOF=1
SEXOS_QUIL_UNDO_REDO_KEY_PROOF=1
SEXOS_APP_LIFECYCLE_CLOSE_RESTORE_PROOF=1
SEXOS_SPINDLE_LIFECYCLE_HELP_V2_PROOF=1
```

## New Gates
| Gate | Evidence | Result |
|------|----------|--------|
| `quil_undo_redo` | `[quil.undo_redo.proof.done] ok=1` | 57 undo pushes across all proofs |
| `quil_undo_redo_key` | `[quil.undo_redo.key.proof.done] ok=1` | Ctrl+Z/Ctrl+Y keybindings |
| `app_lifecycle_close_restore` | `[app.lifecycle.close_restore.proof.done] ok=1` | 4 transitions |
| `spindle_lifecycle_help_v2` | `[spindle.lifecycle.help.proof.done] ok=1` | Lifecycle help section |

## Gate Progression
```
V1-V6: 36 gates
V7:    39 gates (+quil_editor_keybindings, +app_lifecycle_state, +spindle_app_lifecycle)
V8:    43 gates (+quil_undo_redo, +quil_undo_redo_key, +app_lifecycle_close_restore, +spindle_lifecycle_help_v2)
```

## Build + Proof Result
- `entrypoint_build.sh` PASS (9s)
- `run_daily_driver_proof.sh` PASS: 40/43 gates proven, 3 SKIP (Linen QEMU timing), 0 faults
- 57 undo_push calls across all proofs (undo ring exercised by every Quil proof)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ✅ All existing V1-V7 gates preserved
- ✅ Quil undo ring: static BSS, 16-entry circular, 8,448 bytes
- ✅ Undo instrumentation transparent to existing proofs
- ✅ Linen timing skips: 3 gates (same as pre-fix QEMU variance, not a V8 regression)

## Known Limitations
- 3 Linen gates occasionally SKIP (QEMU timing variance)
- Undo ring depth shared across all Quil proofs
- Ctrl+Z/Ctrl+Y synthetic only (modifier tracking not implemented)

## Future Follow-up
- Modifier tracking for real Ctrl+Z/Ctrl+Y
- Deeper undo ring (32 or 64 entries)
- Visual undo depth indicator
- Linen proof ordering defensive re-check
