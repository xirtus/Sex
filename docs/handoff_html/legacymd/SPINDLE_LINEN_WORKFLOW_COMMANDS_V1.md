# SPINDLE_LINEN_WORKFLOW_COMMANDS_V1 — Handoff

## Goal
Add Spindle commands that explain or trigger safe Linen object workflow
operations.  Since Spindle cannot cross-PD create/tag/search Linen objects
(no matching PDX opcodes), commands return honest shortcuts with exact blockers.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `apps/spindle/src/main.rs` | 4 new dispatch arms, proof gate, auto-execute proof | +56 |

## Commands Added

| Command | Behaviour | ok | Reason |
|---------|-----------|----|--------|
| `object-new <name>` | Cannot create Linen objects cross-PD | 0 | No OP_LINEN_CREATE_OBJECT_ASYNC opcode |
| `object-tag <id> <tag>` | Cannot tag Linen objects remotely | 0 | Tag table is local BSS, no PDX tag opcode |
| `object-search <token>` | Cannot search Linen objects remotely | 0 | Search is local in-memory scan, no PDX opcode |
| `linen-search` | Audit: documents exact ABI blocker | 0 | Needs OP_LINEN_SEARCH_OBJECTS (new ABI) |

All commands direct users to the silk-shell Linen surface (Alt+digit) for
keyboard workflow.

## Markers (serial)
```
[spindle.linen.workflow.command] name=NAME ok=N reason=...
[spindle.linen.search.send] token=NAME status=N err=N
[spindle.linen.workflow.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_SPINDLE_LINEN_WORKFLOW_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `spindle_linen_workflow`: PASS (4 commands)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No cross-PD calls attempted — pure local dispatch
- ❌ No fake success — all commands honestly report ok=0 with blocker reason
- ✅ Uses existing command dispatch infrastructure
- ✅ Existing `linen-open`, `linen-list`, `linen-status` commands unchanged

## Known Limitations
- All 4 commands return ok=0 (cannot execute cross-PD)
- No OP_LINEN_SEARCH_OBJECTS opcode exists in Linen server
- Tag table is local BSS in Linen PD — no remote access path
- No async CREATE opcode (existing OP_LINEN_CREATE_OBJECT returns sync reply)

## Future Follow-up
- OP_LINEN_SEARCH_OBJECTS opcode in Linen server (new ABI)
- OP_LINEN_TAG_OBJECT opcode for remote tag assignment
- Async CREATE variant with reply_ring completion
- Cross-PD workflow orchestration via silk-shell bridge
