# QUIL_CURSOR_NAV_TEXT_BUFFER_V1 — Handoff

## Goal
Add cursor left/right/home/end navigation to Quil's text buffer.  Track cursor
position independently from buffer length.  Prove via synthetic movement exercise.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/quil/src/main.rs` | Cursor position var, 4 scancode handlers, cursor proof gate + function | +70 |

## Architecture
- **Cursor variable**: `QUIL_CURSOR_POS: usize` — tracks position in `QUIL_BUFFER`
- **Scancode handlers** (text edit mode only, palette off):
  - `0x4B` (Left Arrow): cursor--, clamped at 0
  - `0x4D` (Right Arrow): cursor++, clamped at buffer length
  - `0x47` (Home): cursor = 0
  - `0x4F` (End): cursor = buffer length
- **Marker**: `[quil.cursor.move] old=N new=N len=N dir=NAME ok=N`

## Key Bindings (Text Mode)
| Key | Scancode | Action |
|-----|----------|--------|
| Left Arrow | 0x4B | Cursor left (clamped min 0) |
| Right Arrow | 0x4D | Cursor right (clamped max len) |
| Home | 0x47 | Cursor to position 0 |
| End | 0x4F | Cursor to buffer end |

## Proof (5-stage synthetic exercise)
1. Seed buffer with "AB" (cursor at pos 2)
2. Left arrow: 2→1
3. Right arrow: 1→2
4. Home: 2→0
5. End: 0→2
6. Left at boundary: 0→0 (clamped)

## Markers (serial)
```
[quil.cursor.move] old=N new=N len=N dir=NAME ok=N
[quil.cursor.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_QUIL_CURSOR_NAV_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `quil_cursor_nav`: PASS (5 cursor moves)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No palette/dispatch changes in palette-active mode
- ✅ Cursor bounded to [0, QUIL_BUFFER_LEN]
- ✅ Existing text buffer edit and HID stash/replay paths preserved
- ✅ Scancode set 1 only (QEMU USB keyboard)

## Known Limitations
- Cursor position not rendered (no visual cursor indicator on screen)
- Scancodes 0x4B/0x4D/0x47/0x4F may differ on real hardware (set 1 vs set 2)
- No insert-at-cursor in text edit mode — append still goes to end
- No Shift+arrow selection
- No Ctrl+arrow word navigation

## Future Follow-up
- Visual cursor indicator (inverted character or underline rect on display)
- Insert-at-cursor for text editing (not just append-to-end)
- Shift+arrow text selection
- Word-level cursor navigation (Ctrl+Left/Right)
- Real hardware scancode compatibility matrix
