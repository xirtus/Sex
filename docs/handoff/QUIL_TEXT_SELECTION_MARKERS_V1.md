# QUIL_TEXT_SELECTION_MARKERS_V1 — Handoff

## Goal
Add selection range markers and proof for Quil's in-memory text buffer.
Track [start, end] range, prove via synthetic exercises.  No visual selection
rendering.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/quil/src/main.rs` | Selection start/end vars, selection proof gate + function | +30 |

## Architecture
- **Selection vars**: `QUIL_SEL_START`, `QUIL_SEL_END` — usize, bounded to buffer
- **Proof**: Seeds "HELLO\nWORLD", exercises 3 selection scenarios:
  1. Select "HELLO" (bytes 0..5)
  2. Select "WORLD" (bytes 6..11)
  3. Empty selection (start==end at pos 3)
- **Marker**: `[quil.text.selection] start=N end=N len=N ok=N`

## Proof Stages
| Stage | Start | End | Buffer Content | Notes |
|-------|-------|-----|----------------|-------|
| 0 | 0 | 5 | HELLO\nWORLD | Select first word |
| 1 | 6 | 11 | HELLO\nWORLD | Select second word (crosses \n) |
| 2 | 3 | 3 | HELLO\nWORLD | Empty selection (cursor only) |

## Markers (serial)
```
[quil.text.selection] start=N end=N len=N ok=N
[quil.text.selection.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_QUIL_TEXT_SELECTION_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `quil_text_selection`: PASS (3 selection markers)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No visual rendering — markers only
- ✅ Selection range bounded to [0, QUIL_BUFFER_LEN]
- ✅ Existing cursor nav and text edit paths unchanged

## Known Limitations
- Selection is in-memory markers only — no visual highlight on screen
- No Shift+arrow to extend selection (scancode modifier not tracked)
- No copy/cut/delete-selection operations wired to selection range
- Selection range not validated against actual buffer state

## Future Follow-up
- Visual selection highlight (inverted color rect on display)
- Shift+arrow selection extension
- Copy/cut/delete-selection using QUIL_SEL_START/END range
- Select-all, select-word, select-line commands
