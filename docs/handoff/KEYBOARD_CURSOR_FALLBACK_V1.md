# KEYBOARD_CURSOR_FALLBACK_V1

**Date:** 2026-05-04
**Status:** IMPLEMENTED

## Context

QEMU 11.0.0 host pointer backend does not deliver nonzero USB HID mouse/tablet
motion. Both usb-mouse and usb-tablet produce only idle reports on real desktop
with physical trackpad. This blocks Silk DE development that requires cursor
interaction.

This fallback maps keyboard keys (arrow keys / WASD) to EV_REL cursor movement
through the existing sexinput -> shell -> display pipeline.

## Compile-Time Gate

- Enable: `SEXOS_KEYBOARD_CURSOR=1` at build time
- Default (unset): no behavior change, zero overhead
- Build command: `SEXOS_KEYBOARD_CURSOR=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh`

## Implementation

**File: servers/sexinput/src/main.rs**

Adds after existing EV_KEY forwarding (line ~258):
- Gate check: `if KEYBOARD_CURSOR_ENABLED && value == 1`
- Key mapping (8px step):
  - 0x11 (W) / 0x48 (Up):    dx=0,  dy=-8
  - 0x1F (S) / 0x50 (Down):  dx=0,  dy=8
  - 0x1E (A) / 0x4B (Left):  dx=-8, dy=0
  - 0x20 (D) / 0x4D (Right): dx=8,  dy=0
- Emits EV_REL to shell: `pdx_call(SLOT_SHELL, OP_HID_EVENT, dx, dy, EV_REL)`
- Arrow keys also continue to send EV_KEY (for shell surface movement shortcuts)

## Pipeline

```
key press -> kernel PS/2 raw input (slot 3)
  -> sexinput.scancode: EV_KEY forwarded to shell (unchanged)
  -> sexinput.keyboard_cursor: EV_REL emitted to shell (new, gated)
    -> silk-shell EV_REL handler: updates POINTER_X/Y
    -> silk-shell sends OP_SURFACE_UPDATE to display
    -> sexdisplay draws cursor at new position
```

## Bounded One-Shot Self-Test

When KEYBOARD_CURSOR_ENABLED is set, sexinput fires a single EV_REL(0, -8)
at boot to prove the full cursor pipeline without requiring QEMU host input
routing. This is necessary because QEMU 11.0.0 on this host does not deliver
ANY external input events to emulated devices (see HOST_INPUT_BACKEND_AUDIT_V1).

**Self-test characteristics:**
- Fires exactly once (one-shot KBD_SELF_TEST_DONE flag)
- Emits EV_REL(0, -8) via the same pdx_call path as real keyboard cursor movement
- Does not repeat (no 120-tick cycle)
- Disabled when gate is unset
- Clearly labeled [keyboard_cursor.self_test]

### Proof markers (headless, -display none):

```
[keyboard_cursor.gate] enabled=1 source=env       ← gate active
[keyboard_cursor.self_test] dx=0 dy=-8             ← self-test fires
[keyboard_cursor.self_test.ok]                      ← self-test complete
[shell.hid.rel.live] n=0 x=0 y=0 dx=0 dy=-8        ← shell receives EV_REL
[shell.cursor.surface.update] n=0 x=640 y=352       ← shell updates cursor
[sexdisplay.cursor.surface.update] n=0 x=640 y=352  ← display receives update
[sexdisplay.cursor.draw] n=0 x=640 y=352            ← cursor drawn at y=352
```

## Diagnostic Markers (budget 16 each)

| Marker | Location | Purpose |
|--------|----------|---------|
| [keyboard_cursor.gate] | sexinput | Boot: enabled=1/0 source=env/default |
| [keyboard_cursor.self_test] | sexinput | Self-test EV_REL emission |
| [keyboard_cursor.key] | sexinput | Key press matched, code + dx/dy |
| [keyboard_cursor.emit.rel] | sexinput | EV_REL sent to shell |
| [shell.hid.rel.live] | silk-shell | Shell received EV_REL from sexinput |
| [shell.cursor.surface.update] | silk-shell | Shell sent OP_SURFACE_UPDATE to display |
| [sexdisplay.cursor.surface.update] | sexdisplay | Display received cursor surface update |
| [sexdisplay.cursor.draw] | sexdisplay | Cursor actually drawn to framebuffer |

## Manual Test

```fish
env SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run \
  2>/tmp/kbd-cursor.err | tee /tmp/kbd-cursor.out
```

Expected:
- [keyboard_cursor.gate] enabled=1 source=env
- pressing arrow keys/WASD produces [keyboard_cursor.key] and [keyboard_cursor.emit.rel]
- cursor moves on screen
- clock keeps counting
- no synthetic panels/click-focus
- no faults

Without SEXOS_KEYBOARD_CURSOR=1:
- [keyboard_cursor.gate] enabled=0 source=default
- no [keyboard_cursor.key] or [keyboard_cursor.emit.rel]
- no behavior change from unmodified build

## Constraints Honored

- no_std, no heap allocation in hot path
- No kernel/ABI/PDX changes
- No renderer changes
- sexdisplay/silk-shell changes are diagnostics-only (budgeted markers)
- Existing USB mouse path preserved
- Gate unset = zero overhead (dead-code eliminated by const bool)

## Files Changed

- servers/sexinput/src/main.rs (+148 lines: gate const, boot diagnostic, USB keyboard handler, PS/2 keyboard cursor mapping, bounded self-test, budgeted kbd/rel markers)
- servers/sexdisplay/src/main.rs (+20 lines: budgeted cursor draw + surface update diagnostics)
- servers/silk-shell/src/main.rs (+19 lines: budgeted EV_REL liveness + cursor surface update diagnostics)
- docs/handoff/KEYBOARD_CURSOR_FALLBACK_V1.md (this file)
- CLAUDE.md (diagnostic summaries)

## STOP Conditions

- [x] Builds with gate disabled: no change
- [x] Builds with gate enabled: adds EV_REL emission
- [x] No kernel/PDX/ABI/renderer changes
- [x] sexdisplay/silk-shell changes are diagnostics-only
- [x] Budgeted diagnostic markers (16 each)
- [x] Full pipeline proven: sexinput → shell → display (see proof markers above)
