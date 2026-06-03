# USB Pointer Shell Apply Fix V1

Date: 2026-06-03

## Status: PASS (build verified, live proof pending)

## Root Cause

`silk-shell` `apply_rel_pointer()` had an early-return gate at L8833:

```rust
// ABS tablet mode: REL deltas would fight ABS position authority.
if ABS_SEEN_VALID {
    return (0, 0);
}
```

**`ABS_SEEN_VALID` was always true by the time real USB EV_REL events arrived**, because:

1. `sexinput` fires a one-shot synthetic EV_ABS proof event at boot (line 363:
   `pdx_call(SLOT_SHELL, OP_HID_EVENT, 16000, 12000, EV_ABS)`)
2. This triggers `process_abs_tablet()` in silk-shell, which sets
   `ABS_SEEN_VALID = true` (line 9026)
3. When real USB tablet EV_REL deltas arrive from sexinput,
   `apply_rel_pointer()` immediately returns `(0, 0)` without applying
   the received dx/dy.

**Evidence of prior knowledge**: The synthetic drag proof at lines 18953-18956
temporarily clears `ABS_SEEN_VALID` before sending EV_REL, then restores it —
a workaround that proves the team knew about this gate.

## Fix

**Removed 4 lines** from `apply_rel_pointer()`: the `ABS_SEEN_VALID` early-return
gate (lines 8832-8835 in original).

**Rationale**: The USB tablet input architecture was redesigned — sexinput now
converts absolute tablet reports to bounded relative deltas (EV_REL) exclusively.
There are no real hardware EV_ABS events reaching the shell. The gate was
obsolete and actively blocking all live cursor movement.

## Files Changed

- `servers/silk-shell/src/main.rs` — removed ABS_SEEN_VALID gate in
  `apply_rel_pointer()` (4 lines removed, 3-line comment added)

## Diff Summary

```diff
-    // ABS tablet mode: REL deltas would fight ABS position authority.
-    if ABS_SEEN_VALID {
-        return (0, 0);
-    }
-
     // ── Conservative REL transfer (no acceleration) ──
+    // (ABS_SEEN_VALID gate removed: USB tablet path now exclusively uses
+    // EV_REL abs-to-rel deltas from sexinput, so REL deltas must never be
+    // blocked.  See USB_POINTER_SHELL_APPLY_FIX_V1.)
```

## Proof Markers Expectation

After fix, live run should show:

```
[usb.pointer.shell.recv.evrel] dx=-64 dy=64 ok=1
[usb.pointer.shell.apply] x=606 y=281 dx=-18 dy=18 ok=1
[usb.pointer.cursor.bounds] x=606 y=281 ok=1
[usb.hid.pointer.click.recv] left=1 value=1 ok=1
```

Note: dx changes from 0 to NONZERO (filtered through transfer_axis).
x/y change from (624, 263).
Cursor surface updates to new position.

## Files NOT Touched

- sexdisplay — no changes needed
- sexinput — producer already emits correct EV_REL
- sexusb — HID reports already arriving
- kernel/ABI/PDX — no changes needed

## Deferred Work / STOP FIRST Notes

- The synthetic EV_ABS proof at sexinput L363 sets ABS_SEEN_VALID and is
  now harmless (it only affects the `pointer_ready` check for click-to-focus,
  which is a separate, correct use of the flag)
- The drag proof workaround at L18953-18956 (temp clear/restore ABS_SEEN_VALID)
  is now dead code but harmless; not removed to keep the diff minimal and
  avoid mixed refactor
- PS/2 mouse IRQ12 remains unimplemented per constraints

## Build

```
./scripts/entrypoint_build.sh → clean, ISO produced
```

## Live Proof Command

```
QEMU_PRINT_CMD=0 SEXUSB_QEMU_DEVICE=tablet-display-sdl SEXOS_QEMU_DISPLAY=sdl ./dev.sh 2>&1 | tee /tmp/sexos_usb_pointer_shell_apply_fix.log
```

## Fault Scan Command

```
rg -n "usb.pointer.shell.recv.evrel|usb.pointer.shell.apply|usb.pointer.cursor.bounds|usb.hid.pointer.click.recv|#PF|#GP|panic|fault.kill" /tmp/sexos_usb_pointer_shell_apply_fix.log logs/qemu-latest.log 2>/dev/null | tail -500
```

## Backup

- `servers/silk-shell/src/main.rs.bak.shell_apply_fix_v1`
