# QUIL_TEXT_BUFFER_COMMANDS_V1 — Handoff

## Goal
Add simple editor command exercises to the Quil proof: clear buffer, type
phrase, show summary (bytes/lines/cursor), cursor tracking via backspace.
No palette or display changes.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/quil/src/main.rs` | Commands proof gate, inline proof block in _start | +39 |

## Commands Exercised
| Command  | Action | Marker |
|----------|--------|--------|
| `clear`  | Zero the mutable buffer (`QUIL_BUFFER_LEN = 0`) | `[quil.text.command] name=clear` |
| `type`   | Append "HELLO\nQUIL" via `text_buffer_append()` | `[quil.text.command] name=type` |
| `summary`| Emit `[quil.text.summary]` with bytes/lines/cursor | `[quil.text.command] name=summary` |
| `cursor` | Backspace 3 chars, emit updated summary | `[quil.text.command] name=cursor` |

## Markers (serial)
```
[quil.text.command] name=NAME ok=N reason=...
[quil.text.summary] bytes=N lines=N cursor=N
[quil.text.command.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_QUIL_TEXT_COMMANDS_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `quil_text_commands`: PASS (4 commands)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No palette interaction — proof operates directly on buffer
- ❌ No HID stash/replay disruption
- ✅ Existing text buffer proof and save/load paths unchanged
- ✅ Buffer bounded to 512 bytes

## Known Limitations
- Commands are synthetic boot-only (no user keybindings)
- No "undo" command
- No clipboard copy/paste
- Summary cursor always equals buffer length (append-only model)

## Future Follow-up
- User-accessible keybindings for clear/summary (e.g., Ctrl+L, Ctrl+G)
- Undo ring (bounded static array)
- Cursor movement within buffer (arrow keys in text mode)
- Selection-based copy (Shift+arrows)
