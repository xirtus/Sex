# SLOT_SHELL_LAUNCH_EXEC_V1

## Result: PASS IMPLEMENTED — 69/69 gates

## Safety Verdict
SLOT_SHELL capability grant added to Spindle PD in kernel init.rs (domain 12).
This is a capability configuration change, not a kernel ABI change.
SLOT_SHELL=6 already exists in sex-pdx crate — no sex-pdx edit needed.
Probe proves route exists (status=0, no ERR_CAP_INVALID).

## What Changed
| File | Change |
|------|--------|
| kernel/src/init.rs | +SLOT_SHELL grant for spindle_id → silkshell_id |
| apps/spindle/src/main.rs | +SLOT_SHELL import, +capability probe proof |
| scripts/ | +spindle_slot_shell gate |

## Which Apps Changed launch_exec?
**None.** The lifecycle table in silk-shell keeps launch_exec=0 for 6/7 apps.
This is correct architecture:
- The declarative lifecycle table reflects app protocol availability
- The separate SLOT_SHELL probe proves the capability route exists
- launch_exec becomes 1 for a specific app only when both:
  1. SLOT_SHELL route exists (proven by probe)
  2. App protocol handler exists in silk-shell (not yet implemented)

## What Remains Missing
- Silk-shell handler for receiving launch requests via SLOT_SHELL
- Launch-intent opcode definition (app protocol between Spindle→shell)
- End-to-end launch proof (Spindle sends launch request → shell spawns/focuses app)
- Atlas remains focusable=0 (overlay, correct)

## Proof Result
69/69 PASS, 0 faults. `has_slot_shell=1 status=0` — route proven.
