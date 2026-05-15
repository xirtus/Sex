# APP_LIFECYCLE_STATE_MATRIX_V1 — Handoff

## Goal
Emit structured app lifecycle state markers for launcher-visible apps from
silk-shell.  Defines state taxonomy (running/ready/deferred/closed) and
focusability per app.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/silk-shell/src/main.rs` | Lifecycle proof gate, function (7 apps), wiring in main loop | +28 |

## Lifecycle State Taxonomy

| State | Meaning |
|-------|---------|
| `running` | App PD is spawned and active (Spindle only) |
| `ready` | App surface exists, keyboard nav proven, focusable |
| `deferred` | App registered but not yet provable (Pointer) |
| `closed` | App surface destroyed or PD terminated |

## Matrix (7 Apps)
| App | sid | State | Focusable | Launch |
|-----|-----|-------|-----------|--------|
| Spindle | 0 | running | yes | active |
| Quil | 201 | ready | yes | palette_owned |
| Linen | 200 | ready | yes | palette_owned |
| Bell | 0 | ready | yes | palette_owned |
| Atlas | 0 | ready | yes | palette_owned |
| Collar | 0 | ready | yes | palette_owned |
| Mesh | 0 | ready | yes | palette_owned |

## Markers (serial)
```
[app.lifecycle.state] app=NAME sid=N state=NAME focusable=N ok=N
[app.lifecycle.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_APP_LIFECYCLE_STATE_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `app_lifecycle_state`: PASS (7 states)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No new PDX opcodes — markers only
- ✅ Static table — no runtime state tracking
- ✅ Emitted from silk-shell main loop (one-shot via done flag)

## Known Limitations
- Lifecycle states are synthetic — not derived from actual PD/surface state
- No runtime transition tracking (ready→running, running→closed)
- sid=0 for Bell/Atlas/Collar/Mesh (surfaces exist but ID not tracked here)
- No `closed` state exercised (all apps are alive in daily driver boot)

## Future Follow-up
- Runtime lifecycle tracking from PD spawn/terminate events
- Surface ID discovery for Bell/Atlas/Collar/Mesh
- Lifecycle transition markers (opened, focused, blurred, closed)
- PDX opcode for silk-shell → app lifecycle query
