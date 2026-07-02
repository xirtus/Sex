# QUIL_UNDO_REDO_KEYBINDINGS_V1 — Handoff

## Goal
Document undo/redo key bindings for Quil.  Since modifier tracking is not
available (scancode set 1, no Shift/Ctrl state), key bindings are synthetic
proof markers only — no real scancode dispatch.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/quil/src/main.rs` | Undo/redo key proof gate + function (2 markers) | +16 |

## Key Bindings (Synthetic)
| Key | Action | Status |
|-----|--------|--------|
| Ctrl+Z | undo | Synthetic proof only (no modifier tracking) |
| Ctrl+Y | redo | Synthetic proof only (no modifier tracking) |

## Why Synthetic
Quil's `scancode_to_char()` maps scancodes to ASCII but does not track modifier
state (Ctrl, Shift, Alt).  Scancode set 1 encodes modifiers as separate
scancodes (Ctrl=0x1D, Shift=0x2A/0x36) that arrive as independent HID events.
Binding Ctrl+Z requires:
1. Track Ctrl press/release state across HID events
2. On 'Z' scancode (0x2C), check if Ctrl is held
3. Dispatch to `text_buffer_undo()` instead of `text_buffer_append('Z')`

This modifier tracking is not yet implemented.  The proof documents the intent.

## Markers (serial)
```
[quil.undo.key] key=Ctrl+Z action=undo ok=N reason=static_ring_restore
[quil.redo.key] key=Ctrl+Y action=redo ok=N reason=static_ring_replay
[quil.undo_redo.key.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_QUIL_UNDO_REDO_KEY_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `quil_undo_redo_key`: PASS

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No scancode dispatch changes — proof markers only
- ✅ Undo/redo functions exist and are proven (QUIL_UNDO_REDO_STATIC_RING_IMPL_V1)

## Known Limitations
- Modifier tracking not implemented (Ctrl/Shift/Alt state unknown)
- Real Ctrl+Z/Ctrl+Y requires modifier state machine in HID event handler
- Scancode set 2 on real hardware uses different Ctrl/Z codes

## Future Follow-up
- Modifier tracking state machine in `quil_dispatch_palette_key`
- Real Ctrl+Z → `text_buffer_undo()`, Ctrl+Y → `text_buffer_redo()`
- Shift tracking for lowercase input
- Scancode set 1/2 compatibility matrix
