# KEYBOARD_CURSOR_KEYTRACE_V1

## Summary

Added budgeted keytrace marker inside the SEXOS_KEYBOARD_CURSOR=1 debug path.
Fires on every key-down event received by handle_hid_event while the flag is active.
Purpose: diagnose which scancodes actually arrive at the shell — confirm whether
live WASD inputs reach the handler or are intercepted upstream.

## Change

### servers/silk-shell/src/main.rs (~line 9099)

Inserted before the movement match, inside `if KEYBOARD_CURSOR_DEBUG_ENABLED`:

```rust
static mut KEYTRACE_BUDGET: u32 = 64;
if KEYTRACE_BUDGET > 0 {
    KEYTRACE_BUDGET -= 1;
    serial_println!(
        "[keyboard.cursor.debug.keytrace] key={:#04x} pressed={} ok=1",
        scancode, value
    );
}
```

Budget: 64 events. Budget exhaustion = silent (no FAIL).
`pressed` always `1` here (marker is inside `if value == 1 {}` block).

## Expected Output (WASD working)
```
[keyboard.cursor.debug.keytrace] key=0x11 pressed=1 ok=1   ← W
[keyboard.cursor.debug.keytrace] key=0x1e pressed=1 ok=1   ← A
[keyboard.cursor.debug.keytrace] key=0x1f pressed=1 ok=1   ← S
[keyboard.cursor.debug.keytrace] key=0x20 pressed=1 ok=1   ← D
```

## Diagnostic Paths

If keytrace markers appear but no `move` markers:
→ Scancode values differ from 0x11/0x1E/0x1F/0x20. Check actual key values and patch match arm.

If NO keytrace markers appear at all:
→ handle_hid_event not receiving keyboard events in live mode. Check event routing:
  main loop uses OP_HID_EVENT path, not handle_hid_event for normal keyboard.
  Cursor debug may need to be added to main event loop EV_KEY handler too.

If keytrace appears on Enter(0x1C) only:
→ Other keys intercepted before handle_hid_event (atlas, command palette, etc.).

## Backup
- `servers/silk-shell/src/main.rs.bak.keytrace_v1`
