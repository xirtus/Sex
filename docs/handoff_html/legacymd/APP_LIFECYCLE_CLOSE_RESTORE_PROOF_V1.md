# APP_LIFECYCLE_CLOSE_RESTORE_PROOF_V1 — Handoff

## Goal
Prove lifecycle transition markers for close/minimize/restore/hide/show states.
Synthetic markers only — no destructive close of core apps.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/silk-shell/src/main.rs` | Lifecycle close/restore proof gate + function + wiring | +25 |

## Transition Matrix
| App | Old State | New State | Trigger |
|-----|-----------|-----------|---------|
| Quil | ready | minimized | Synthetic minimize |
| Quil | minimized | restored | Synthetic restore |
| Linen | ready | hidden | Synthetic hide |
| Linen | hidden | visible | Synthetic show |

## Lifecycle State Taxonomy
```
running → ready → minimized/hidden → closed
              ↑        ↓              (terminal)
              └── restore/show ──────┘
```

## Markers (serial)
```
[app.lifecycle.transition] app=NAME old=NAME new=NAME ok=N reason=...
[app.lifecycle.close_restore.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_APP_LIFECYCLE_CLOSE_RESTORE_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `app_lifecycle_close_restore`: PASS (4 transitions)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No destructive close of core apps (synthetic markers only)
- ❌ No surface destruction or PD termination
- ✅ All transitions are synthetic proof markers

## Known Limitations
- Transitions are synthetic — no actual surface state changes
- No `closed` state exercised (no PD termination)
- No save-on-close workflow (buffer state lost on close)
- No minimize animation or visual feedback

## Future Follow-up
- Real minimize/restore via surface hide/show opcodes to sexdisplay
- Real close with PD termination (requires kernel spawn tracking)
- Save-on-close (auto-save Quil buffer to RamFS before close)
- Visual transition effects (fade, slide via Silk renderer)
