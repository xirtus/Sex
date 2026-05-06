# QUIL_INITIAL_BOOT_DRAW_V1

Status: implemented (minimal patch)

## Root Cause

Quil surface (201) was focused by shell at boot, but Quil did not issue any initial
0xEF fill before entering `pdx_listen_raw(0)`. Its first visible draw only happened
on key events (`OP_HID_EVENT`), causing a blank-ish partial UI state despite alive
clock/render loop.

## Change

File changed:
- `servers/quil/src/main.rs`

Exact boot draw path added:
- In `_start()`, before `serial_println!("[quil.ready]")` and before listen loop:
  - `pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_QUIL, 0, (color<<32)|(h<<16)|w)`
  - rect: local full-surface style `640x480`
  - color: `0x001F2A44`
  - marker: `[quil.boot.draw.ok]`

No ABI/slot/opcode changes.
No kernel/display/shell policy changes.
No framebuffer ownership changes.

## Expected Markers

- `[silk-shell] Boot 0xEC surface 201 (Quil) created`
- `[silk-shell] Boot 0xEC surface 200 (Linen) created`
- `[quil.boot.draw.ok]`
- `[linen] Fill rect 0xEF sent to sexdisplay`
- `[silk-shell.ui.ready] surfaces=2`

## Remaining Gap

If UI is still partial after this patch, next audit owner is shell frame/lifecycle
visibility ordering (not ABI/slot grants), because Quil now paints at boot.
