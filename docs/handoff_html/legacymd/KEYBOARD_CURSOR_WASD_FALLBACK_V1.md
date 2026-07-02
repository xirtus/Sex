# KEYBOARD_CURSOR_WASD_FALLBACK_V1

## Summary

Extended debug keyboard cursor fallback to include WASD scancodes.
Arrow keys were unreliable in live QEMU test (extended scancode mismatch suspected).
WASD are normal PS/2 letter scancodes — reliable in all QEMU keyboard modes.

## Changes

### servers/silk-shell/src/main.rs (~line 9102)

Outer match arm extended:
```
0x4B | 0x4D | 0x48 | 0x50 | 0x11 | 0x1E | 0x1F | 0x20
```

Inner dx/dy match:
```
0x4B | 0x1E => (-CURSOR_STEP, 0)  // Left arrow / A
0x4D | 0x20 => (CURSOR_STEP, 0)   // Right arrow / D
0x48 | 0x11 => (0, -CURSOR_STEP)  // Up arrow / W
0x50 | 0x1F => (0, CURSOR_STEP)   // Down arrow / S
```

Step remains 32px. Proof markers, click path, statics — all unchanged.

## Key Note: WASD consumed before Spindle passthrough

W(0x11), A(0x1E), S(0x1F), D(0x20) are all in `is_spindle_text_key` range.
The cursor debug block runs BEFORE the Spindle passthrough check and uses `return`.
When SEXOS_KEYBOARD_CURSOR=1: WASD = cursor, not text input to apps.

## Proof Markers (same as before)
```
[keyboard.cursor.debug.begin] ok=1
[keyboard.cursor.debug.move] key=0x11 dx=0 dy=-32 old_x=640 old_y=360 new_x=640 new_y=328 ok=1
[keyboard.cursor.debug.done] ok=1
```

## Backup
- `servers/silk-shell/src/main.rs.bak.wasd_v1`
