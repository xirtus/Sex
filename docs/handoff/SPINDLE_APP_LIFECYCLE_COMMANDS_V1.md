# SPINDLE_APP_LIFECYCLE_COMMANDS_V1 — Handoff

## Goal
Add Spindle command `app-state` that displays the app lifecycle state matrix:
which apps are running/ready/deferred, their surface IDs, focusability, and
launch method.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `apps/spindle/src/main.rs` | `app-state` dispatch arm, lifecycle proof gate | +24 |

## Command Added
| Command | Behaviour | ok |
|---------|-----------|----|
| `app-state` | Renders 7-row lifecycle state matrix with sid/state/focus/launch | 1 |

Output format:
```
App Lifecycle State Matrix:
  app     sid   state     focusable  launch
  Spindle 0     running   yes        active
  Quil    201   ready     yes        palette_owned
  Linen   200   ready     yes        palette_owned
  Bell    0     ready     yes        palette_owned
  Atlas   0     ready     yes        palette_owned
  Collar  0     ready     yes        palette_owned
  Mesh    0     ready     yes        palette_owned
Lifecycle states: running > ready > deferred > closed.
Focus: Alt+1-7 or app launcher (silk-shell palette).
Cross-PD spawn: blocked (SLOT_SHELL needed).
```

## Markers (serial)
```
[spindle.lifecycle.command] name=NAME ok=N reason=...
[spindle.lifecycle.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_SPINDLE_APP_LIFECYCLE_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `spindle_app_lifecycle`: PASS (1 command)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No cross-PD queries — static local table only
- ✅ Uses existing command dispatch infrastructure
- ✅ Consistent with `app_registry_static` and `app_launch_commands` data

## Known Limitations
- Static matrix — no live silk-shell lifecycle query
- sid=0 for apps without known surface IDs (Bell/Atlas/Collar/Mesh)
- No runtime state transitions visible from Spindle
- Launch method is always palette_owned (honest blocker)

## Future Follow-up
- Live lifecycle query via PDX opcode to silk-shell
- Runtime state transition display (polling or event-driven)
- Surface ID discovery for all apps
- `app-focus <name>` command to request focus change
