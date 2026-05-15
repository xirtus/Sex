# SPINDLE_EDITOR_COMMANDS_V2 — Handoff

## Goal
Update Spindle's edit-help and edit-status commands to reference new Quil V5/V6
features: cursor navigation, text selection markers, and delete commands.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `apps/spindle/src/main.rs` | Updated edit-help/edit-status commands, editor V2 proof gate | +51 |

## Changes

### edit-help (V1 → V2)
- Added: Cursor Nav section (Left/Right/Home/End with scancodes)
- Added: Delete commands (Delete, Ctrl+K, Ctrl+Y)
- Added: Selection range mention
- Removed: "no cursor nav, no selection" limitation text
- Marker changed from `spindle.quil.workflow.command` → `spindle.editor.command`

### edit-status (V1 → V2)
- Added: cursor navigation line (V5 feature)
- Added: text mode delete char/eol/line (V6 feature)
- Added: selection range markers (V6 feature)
- Updated: proof gates list (commands V1, cursor V1, select V1, delete V1 PASS)
- Marker changed from `spindle.quil.workflow.command` → `spindle.editor.command`

## Editor V2 Proof
Auto-executes `edit-help` and `edit-status` at boot, emitting V2 markers.

## Markers (serial)
```
[spindle.editor.command] name=NAME ok=N reason=...
[spindle.editor.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_SPINDLE_EDITOR_V2_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `spindle_editor_v2`: PASS (4 editor commands)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No cross-PD calls — pure informational dispatch
- ✅ Existing Quil workflow commands (quil, edit) unchanged
- ✅ Backward compatible — previous markers still emitted by V5 proof

## Known Limitations
- Help text mentions keyboard bindings not yet wired to scancodes (Ctrl+K, Ctrl+Y, Delete)
- No live Quil state readback (static help only)
- Editor versioning is manual (no runtime feature detection)

## Future Follow-up
- Wire Ctrl+K/Ctrl+Y/Delete scancodes to Quil delete functions
- Runtime feature detection (query Quil for supported commands)
- Auto-generated help from Quil's actual dispatch table
