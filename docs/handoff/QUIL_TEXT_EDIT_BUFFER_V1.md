# QUIL_TEXT_EDIT_BUFFER_V1 — Handoff

## Goal
Make Quil more editor-like: in-memory text buffer accepting keyboard characters,
backspace deletion, and Enter for newlines.  Preserves HID stash/replay.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/quil/src/main.rs` | Scancode→ASCII map, 3 buffer fns, dispatch routing, 14-stage proof | +290 |

## New Functions

| Function | Role |
|----------|------|
| `scancode_to_char(scancode)` | Map 40+ scancode-set-1 codes to ASCII (A-Z, 0-9, punctuation, space) |
| `text_buffer_append(ch)` | Append one byte to `QUIL_BUFFER`; rejects control chars except `\n` |
| `text_buffer_backspace()` | Delete last byte; no-op if empty |
| `text_buffer_newline()` | Append `\n`; counts lines |

## Dispatch Changes
- **Enter** (palette off): calls `text_buffer_newline()` + redraw
- **Esc** (palette off): toggles palette ON (was: "reject inactive")
- **Default keys** (palette off): calls `scancode_to_char()` → `text_buffer_append()` + redraw
- **Backspace** (palette off): calls `text_buffer_backspace()` + redraw
- Palette ON behaviour: unchanged (nav/select/liveness color toggle)

## Proof (14 stages, stash/replay)
1. Seed `H`, `e`, `l`, `l`, `o` into HID stash
2. Seed `Enter` (newline)
3. Seed `Q`, `u`, `i`, `l`
4. Seed `Backspace` (deletes `l`)
5. Seed `1` (`!`), `Enter`
6. Turn palette off, replay stash → buffer edits
7. Verify buffer length + line count

## Markers (serial)
```
[quil.text.recv] code=N ch=N ok=N
[quil.text.append] len=N ch=N
[quil.text.backspace] old=N new=N ok=N
[quil.text.enter] line=N len=N ok=N
[quil.text.buffer.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_QUIL_TEXT_BUFFER_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `quil_text_buffer`: PASS (7 recv events)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / display changes
- ❌ No storage save/load for text edits (buffer is in-memory BSS)
- ✅ HID stash/replay preserved — all synthetic events go through stash
- ✅ Buffer bounded to 512 bytes (QUIL_BUFFER_MAX_LEN)
- ✅ Palette mode restored after proof

## Known Limitations
- Shift not tracked — all chars uppercase (scancode set 1, no modifier)
- No cursor movement within buffer (append-only + backspace)
- No selection, copy/paste, undo
- No save of edited text to RamFS (existing save/load uses `QUIL_TEXT_INIT`)

## Future Follow-up
- Cursor movement (arrow keys in text mode)
- Shift modifier tracking for lowercase
- Save edited buffer to RamFS
- Multiple buffers / tab support
