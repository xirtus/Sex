# VISUAL_CURSOR_KEYBOARD_FALLBACK_DEBUG_V1

## Summary

Added debug-only keyboard cursor fallback for live visual testing when QEMU host mouse
produces no USB HID reports. Replaced magenta cursor with white/black-outline cursor.

## Root Cause

QEMU `sdl`/`gtk`/`tablet` display backends do not produce USB HID pointer reports through
the xHCI tablet endpoint in the current QEMU version used. The sexusb driver stalls waiting
for interrupt-IN transfers that never arrive. Cursor stays at 640×360 center.

## Changes

### servers/sexdisplay/src/main.rs
- `CURSOR_ARROW_COLOR`: `0x00FF00FF` (magenta) → `0x00FFFFFF` (white)
- Black outline pass unchanged. Result: white arrow, black halo — readable on any background.
- `[sexdisplay.cursor.visual.contrast]` marker now logs `color=0x00ffffff`.

### servers/silk-shell/src/main.rs
- Added const `KEYBOARD_CURSOR_DEBUG_ENABLED` (compile-time, `option_env!("SEXOS_KEYBOARD_CURSOR")`).
- Added statics: `KEYBOARD_CURSOR_DEBUG_BEGUN`, `KEYBOARD_CURSOR_DEBUG_DONE`, `KEYBOARD_CURSOR_MOVE_BUDGET`.
- Inserted debug cursor handler in `handle_hid_event` (EV_KEY / value==1 block),
  **before** Spindle text key passthrough, **after** command palette intercept.
  Location: ~line 9095 in modified file.

#### Key behavior (SEXOS_KEYBOARD_CURSOR=1 only):
- `0x4B` Left / `0x4D` Right / `0x48` Up / `0x50` Down → move cursor 32px, route via `send_cursor_checked`.
- `0x39` Space / `0x1C` Enter → synthesize BTN_LEFT down+up if `ABS_SEEN_VALID || POINTER_USB_STATE_INIT` (pointer already ready). If not ready, logs `ok=0 reason=pointer_not_ready` — does NOT set POINTER_USB_STATE_INIT.
- Keys consumed with `return` — not propagated to Spindle/Atlas/reserved UI path.
- Atlas mode: safe. Main event loop intercepts arrows for Atlas BEFORE calling handle_hid_event.

#### Proof markers emitted:
```
[keyboard.cursor.debug.begin] ok=1            — first arrow key event, once
[keyboard.cursor.debug.move] key=0x4b dx=-32 dy=0 old_x=640 old_y=360 new_x=608 new_y=360 ok=1
[keyboard.cursor.debug.done] ok=1             — first successful op, once
[keyboard.cursor.debug.click] key=0x39 button=1 ok=1   — or ok=0 reason=pointer_not_ready
```

### scripts/daily_driver_master_gate.sh
- Added `gate_keyboard_cursor_debug="SKIP"` to var declarations.
- Added gate block after `cursor_visual_contrast`:
  - SKIP: no `keyboard.cursor.debug.begin` in log (flag absent or no keys pressed — correct for headless).
  - PASS: begin + move ok=1 + done all present.
  - FAIL: begin present but move or done missing.
- Added `"keyboard_cursor_debug:$gate_keyboard_cursor_debug"` to summary array.

## USB Status (unchanged, honest)
USB producer remains blocked. `sexusb` proves pointer producer blocked on QEMU input boundary.
No USB gates claimed PASS. No change to USB proof markers.

## Live Test Command
```bash
SEXOS_KEYBOARD_CURSOR=1 ./scripts/entrypoint_build.sh && \
SEXOS_QEMU_DISPLAY=sdl SEXUSB_QEMU_DEVICE=tablet \
qemu-system-x86_64 [normal flags] sexos-v1.0.0.iso 2>&1 | tee /tmp/keyboard_cursor_debug_live.log
```
Then press arrow keys — cursor should move in 32px steps.
Space/Enter clicks only if pointer was previously initialized (e.g., via pointer proof sequence).

## Proof Run
```bash
./scripts/run_daily_driver_proof.sh /tmp/keyboard_cursor_debug_v1.log
```
Expected: `cursor_visual_contrast=PASS`, `keyboard_cursor_debug=SKIP` (headless, no keys).

## Fault Scan (clean)
- No new unsafe blocks added.
- `send_cursor_checked` already bounds-clamps. No OOB writes.
- Recursive `handle_hid_event(EV_BTN,...)` safe: EV_BTN path does not re-enter EV_KEY.
- All statics initialize to `false`/`0` — no uninit reads.

## Backups
- `servers/silk-shell/src/main.rs.bak.kbd_cursor_v1`
- `servers/sexdisplay/src/main.rs.bak.kbd_cursor_v1`
- `scripts/daily_driver_master_gate.sh.bak.kbd_cursor_v1`
