# SPINDLE_QUIL_WORKFLOW_COMMANDS_V1 — Handoff

## Goal
Add Spindle commands for Quil editor workflow: help, status, key bindings,
and buffer status.  Informational only — no cross-PD editor control.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `apps/spindle/src/main.rs` | 4 new dispatch arms, proof gate, auto-execute proof | +39 |

## Commands Added
| Command | Behaviour | ok |
|---------|-----------|----|
| `quil` | Quil overview: surface, buffer, palette, save status | 1 |
| `edit` | Editor status: text mode, palette, limitations | 1 |
| `edit-help` | Detailed key bindings: type, backspace, enter, esc, nav | 1 |
| `edit-status` | Buffer status: max bytes, palette commands, proof gates | 1 |

## Markers (serial)
```
[spindle.quil.workflow.command] name=NAME ok=N reason=...
[spindle.quil.workflow.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_SPINDLE_QUIL_WORKFLOW_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `spindle_quil_workflow`: PASS (4 commands)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No cross-PD calls — pure informational dispatch
- ✅ All commands return ok=1 (help/status rendered successfully)
- ✅ Uses existing command dispatch infrastructure

## Known Limitations
- Commands are informational only — cannot control Quil editor remotely
- No live Quil buffer readback (no PDX opcode for buffer query)
- Key bindings listed are static — no runtime state inspection
- No cross-PD save/load trigger from Spindle

## Future Follow-up
- OP_QUIL_BUFFER_STATUS opcode for live buffer readback
- Cross-PD save trigger from Spindle (needs Quil PDX opcode)
- Live cursor position query
- Spindle → Quil text injection for macro/script support
