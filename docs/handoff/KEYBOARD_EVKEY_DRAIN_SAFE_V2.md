# KEYBOARD_EVKEY_DRAIN_SAFE_V2

Date: 2026-05-13

## 1) Pre-linen drain restored
Yes. The `pdx_try_listen_raw(0)` pre-linen bounded drain loop is restored in `servers/silk-shell/src/main.rs`.

## 2) Exact EV_KEY drain handling path
When drain pops `OP_HID_EVENT`, it calls:
- `handle_hid_event(req.arg2, req.arg0, req.arg1)`

`handle_hid_event(...)` now handles EV_KEY directly:
- emits `[silk-shell.hid.recv] ...`
- emits `[silk-shell.key.recv] ...`
- updates local key state for Ctrl/F9 edge
- routes key-down (`value==1`) by focus using existing slots:
  - Quil: `pdx_call(SLOT_QUIL, OP_HID_EVENT, scancode, value, EV_KEY)`
  - Linen: `pdx_call(SLOT_LINEN, OP_HID_EVENT, scancode, value, EV_KEY)`
  - Spindle: `pdx_call(SLOT_SPINDLE, OP_HID_EVENT, scancode, value, EV_KEY)`
- emits existing route markers (`[silk-shell.key.route] ...`)
- returns early for EV_KEY so pointer path is unchanged.

Pointer classes (`EV_ABS`, `EV_REL`, `EV_BTN`) remain in existing code path.

## 3) sexinput status
`[sexinput.key.send]` diagnostics remain intact and unchanged in behavior intent.

## 4) Build proof
- Command: `./scripts/entrypoint_build.sh`
- Result: success (`[SEXOS ENTRYPOINT] success`)
- Optional host warning remains about missing `x86_64-sex` target in the optional preflight check.

## 5) Runtime status
Not executed here (no GTK run in this turn), so runtime proof is pending.
First verification to run:
- keyboard grep lane (ab Backspace Enter)
- tablet sanity grep lane

If keyboard still fails after this patch, first dead hop should be determined by first missing marker in order:
1. `sexinput.key.send`
2. `silk-shell.hid.recv`
3. `silk-shell.key.recv`
4. `silk-shell.key.route`

## Files changed
- `servers/silk-shell/src/main.rs`
- (no new edits in this mission) `servers/sexinput/src/main.rs` retained from prior mission

## Backups
- `/tmp/silk-shell.main.rs.pre_keyboard_evkey_drain_safe_v2.bak`
- `/tmp/sexinput.main.rs.pre_keyboard_evkey_drain_safe_v2.bak`
