# SPINDLE_LIFECYCLE_HELP_V2 — Handoff

## Goal
Update Spindle help text with `lifecycle` command covering close/minimize/restore
states, and mention Quil V8 undo/redo editor features.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `apps/spindle/src/main.rs` | `lifecycle` dispatch arm, lifecycle help V2 proof gate | +22 |

## Command Added
| Command | Behaviour | ok |
|---------|-----------|----|
| `lifecycle` | Renders lifecycle help: states, transitions, keys, limits | 1 |

Output:
```
App Lifecycle Help V2:
  States:     running > ready > minimized > hidden > closed
  Transitions: open, focus, minimize, restore, hide, show, close
  Commands:   app-state (matrix), lifecycle (this help)
  Keys:       Alt+F4 close, Alt+Z zoom, Alt+M minimize
  Editor:     Ctrl+Z undo, Ctrl+Y redo (Quil V8 static ring)
  Restore:    via launcher re-select or Alt+digit palette
  Close:      surface destroyed, state lost (no restore yet)
  Spindle:    always running, self-close returns to launcher
Limitations: no PD persistence across close, no save-on-close.
```

## Markers (serial)
```
[spindle.lifecycle.help] section=lifecycle ok=N
[spindle.lifecycle.help.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_SPINDLE_LIFECYCLE_HELP_V2_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `spindle_lifecycle_help_v2`: PASS

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No cross-PD queries — static help text only
- ✅ Uses existing command dispatch infrastructure
- ✅ Consistent with `app-state` and `lifecycle` state matrix

## Known Limitations
- Help mentions Ctrl+Z/Ctrl+Y but modifier tracking not yet implemented
- Close/restore mentions Alt+F4/Alt+Z which are proven for windows, not apps
- No live state detection (always reports static help)

## Future Follow-up
- Dynamic help that reflects current system state
- app-state auto-detection from silk-shell lifecycle query
- Modifier tracking for real Ctrl+Z/Ctrl+Y in Quil
