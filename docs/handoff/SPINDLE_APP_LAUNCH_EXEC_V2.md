# SPINDLE_APP_LAUNCH_EXEC_V2 — Handoff

## Goal
Audit whether Spindle can actually execute cross-PD app launch from its
terminal environment.  No new ABI, no fake launch success.  If Spindle cannot
call the shell launcher, document the exact blocker.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `apps/spindle/src/main.rs` | Launch exec audit gate, 7-app honest capability check | +23 |

## Architecture
- **Gate**: `SPINDLE_LAUNCH_EXEC_PROOF_ENABLED` via `SEXOS_SPINDLE_APP_LAUNCH_EXEC_PROOF=1`
- **Audit**: Spindle's PD capabilities are checked at boot
- **Spindle caps**: SLOT_DISPLAY(5), SLOT_STORAGE(10), SLOT_BELL(12), SLOT_LINEN(8)
- **Missing**: SLOT_SHELL, kernel spawn capability, launch PDX opcode
- **Result**: All 7 apps (quil, linen, bell, atlas, collar, mesh) are palette-owned — Spindle cannot cross-PD spawn any of them. Spindle itself is already active.

## App Launch Capability Matrix
| App | ok | Reason |
|-----|----|--------|
| spindle | 1 | Already active (self) |
| quil | 0 | Palette-owned, no cross-PD spawn |
| linen | 0 | Palette-owned, no cross-PD spawn |
| bell | 0 | Palette-owned, no cross-PD spawn |
| atlas | 0 | Palette-owned, no cross-PD spawn |
| collar | 0 | Palette-owned, no cross-PD spawn |
| mesh | 0 | Palette-owned, no cross-PD spawn |

## Markers (serial)
```
[spindle.launch.exec.audit] safe=N reason=...
[spindle.launch.exec] app=NAME ok=N reason=...
[spindle.launch.exec.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_SPINDLE_APP_LAUNCH_EXEC_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `spindle_launch_exec`: PASS (7 exec rows)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No new PDX opcodes — audit only
- ❌ No fake launch success — honest "ok=0" for palette-owned apps
- ✅ Existing `launch` command honest-report path unchanged
- ✅ All existing app commands (apps, app-info, app-status) preserved

## Known Limitations
- Spindle has no SLOT_SHELL grant — cannot send launch commands to silk-shell
- No kernel spawn capability from user-space PDs
- No launch-from-Bell dispatch in silk-shell (Bell events not bridged to launcher)
- Spindle's `launch` command remains informational-only

## Future Follow-up
- SLOT_SHELL grant to Spindle's PD (requires Collar policy update)
- Kernel spawn opcode for user-space PDs (major ABI change)
- Launch-intent PDX opcode between Spindle and silk-shell
- Bell-to-launcher event bridge for Spindle-originated launch requests
