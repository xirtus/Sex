# QUIL_TEXT_DELETE_WORD_LINE_V1 — Handoff

## Goal
Add bounded editor delete commands: delete character at cursor, delete to end
of line, delete entire current line.  Prove via synthetic buffer manipulation.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/quil/src/main.rs` | 3 delete functions (char, to_eol, line), delete proof gate + function | +118 |

## New Functions

| Function | Behaviour | Marker |
|----------|-----------|--------|
| `text_buffer_delete_char()` | Remove char at cursor, shift left. No-op if cursor at end. | `mode=char` |
| `text_buffer_delete_to_eol()` | Delete from cursor to next \n (or EOF). No-op if at \n. | `mode=to_eol` |
| `text_buffer_delete_line()` | Delete entire current line (\n-delimited). Clamp cursor. | `mode=line` |

## Implementation Details

### text_buffer_delete_char
- Check: cursor < buffer_len
- Shift: buffer[cursor..len-2] ← buffer[cursor+1..len-1]
- Decrement len, zero-fill freed byte

### text_buffer_delete_to_eol
- Scan forward from cursor to \n or EOF
- Compute del_count = eol - cursor
- Shift remaining buffer left by del_count
- Zero-fill freed bytes

### text_buffer_delete_line
- Scan backward from cursor to \n or BOF (line_start)
- Scan forward from line_start to \n or EOF (line_end)
- Include trailing \n if present
- Shift + zero-fill, clamp cursor if needed

## Proof (3-stage synthetic exercise)
| Stage | Buffer | Cursor | Operation | Expected |
|-------|--------|--------|-----------|----------|
| 0 | ABC\nDEF\nGHI | 0 | delete_char | BC\nDEF\nGHI (len=10) |
| 1 | BC\nDEF\nGHI | 3 | delete_to_eol | BC\n\nGHI (len=7) |
| 2 | BC\n\nGHI | 3 | delete_line | BC\nGHI (len=6) |

## Markers (serial)
```
[quil.text.delete] mode=NAME old=N new=N ok=N
[quil.text.delete.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_QUIL_TEXT_DELETE_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `quil_text_delete`: PASS (3 delete markers)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No display redraw — delete markers only (visual not updated)
- ✅ All functions bounded to QUIL_BUFFER_MAX_LEN (512 bytes)
- ✅ Cursor clamped after delete_line
- ✅ Zero-fill prevents stale data
- ✅ Existing text edit paths unchanged

## Known Limitations
- Delete operations not bound to keyboard scancodes (proof-only)
- No undo capability (deleted data lost)
- No delete word (Ctrl+W) or delete to BOL
- Visual not redrawn after delete (buffer content changes but display stale)

## Future Follow-up
- Bind delete functions to keyboard scancodes (Delete, Ctrl+K, Ctrl+Y)
- Undo ring for reversible delete
- Delete word (to next space/newline boundary)
- Auto-redraw after delete operations
- Delete selection range (when selection is active)
