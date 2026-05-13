# KEYBOARD_SEXINPUT_TO_SHELL_DELIVERY_V1

Date: 2026-05-13

## Exact first dead hop
Dead hop was in `silk-shell` pre-linen input drain:
- The pre-drain loop popped `OP_HID_EVENT` messages with `pdx_try_listen_raw(0)`.
- Popped events were sent to `handle_hid_event(...)`.
- `handle_hid_event(...)` handles `EV_ABS`, `EV_REL`, `EV_BTN` only, not `EV_KEY`.
- Result: keyboard `EV_KEY` could be consumed before main `OP_HID_EVENT` dispatch and never reach `[silk-shell.key.recv]`.

## Fix applied
Applied minimal local fix + diagnostics:

1. `servers/silk-shell/src/main.rs`
- Disabled pre-linen input drain block that popped HID events before main dispatch.
- Added bounded receive marker in main HID path:
  - `[silk-shell.hid.recv] class=N code=N value=N a0=N a1=N a2=N`
- Existing markers preserved.

2. `servers/sexinput/src/main.rs`
- In USB keyboard mapped EV_KEY send path, replaced raw `pdx_call(...)` with existing capability-aware helper `send_shell_hid_event(...)`.
- Added send diagnostics markers:
  - `[sexinput.key.send] code=N down=N mod=N dst=N ok=N err=N`
- Existing markers preserved (`[sexinput.key.emit]` etc).

No kernel / ABI / opcode / renderer / display edits.

## Files changed
- `servers/sexinput/src/main.rs`
- `servers/silk-shell/src/main.rs`

## Build result
- Command: `./scripts/entrypoint_build.sh`
- Result: success (`[SEXOS ENTRYPOINT] success`)
- Note: optional host gate still warns about missing `x86_64-sex` target in this environment.

## Runtime proof markers expected
After boot with GTK USB keyboard lane and typing `ab`, grep should show:
- `[sexinput.key.emit] ...`
- `[sexinput.key.send] ... dst=6 ok=1 err=0`
- `[silk-shell.hid.recv] ... class=EV_KEY ...`
- `[silk-shell.key.recv] ...`

If `silk-shell.key.route` / `spindle.*` remain missing after this, that is the next downstream hop (focus/route stage), not sexinput->shell delivery.
