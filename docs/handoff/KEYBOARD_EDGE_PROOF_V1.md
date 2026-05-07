# KEYBOARD_EDGE_PROOF_V1

- date: 2026-05-07
- proves: sexinput → silk-shell EV_KEY down/up path (non-cursor keys)

## What Was Unproven

KEYBOARD_DEVICE_MODE_V1 proved EV_REL cursor movement from arrow/WASD keys via
USB keyboard. The EV_KEY path (keydown/keyup for non-cursor keys, e.g. Enter,
letters, function keys) had never been traced end-to-end from sexinput to
silk-shell.

## What Was Added

### servers/sexinput/src/main.rs

New one-shot proof block (section 6a), gated by `!SYNTHETIC_INPUT_PROOFS_DISABLED`:

- tick == 3: send `OP_HID_EVENT(0x1C, 1, EV_KEY)` → Enter keydown to SLOT_SHELL
- tick == 4: send `OP_HID_EVENT(0x1C, 0, EV_KEY)` → Enter keyup to SLOT_SHELL

Markers:
```
[sexinput.key.ev_key.down code=0x1c]
[sexinput.key.ev_key.up code=0x1c]
```

Tick timing note: cooperative scheduling runs sexinput at ~0.7 iterations/second.
Tick 3/4 fires within first ~5 seconds of probe. Tick values ≥ 10 do NOT fire
reliably in a 10s probe window.

### servers/silk-shell/src/main.rs

Budgeted receive marker added to EV_KEY dispatch (budget = 4):

```
[shell.key.ev_key.received code=0x1c value=1]
[shell.key.ev_key.received code=0x1c value=0]
```

## Observed Marker Chain

```
[sexinput.key.ev_key.down code=0x1c]
[shell.key.ev_key.received code=0x1c value=1]
[silk-shell.key.route] owner=quil sid=201 scancode=0x1c
[sexinput.key.ev_key.up code=0x1c]
[shell.key.ev_key.received code=0x1c value=0]
```

Confirms:
1. sexinput sends EV_KEY to silk-shell via OP_HID_EVENT ✓
2. silk-shell receives EV_KEY keydown and keyup ✓
3. silk-shell routes key to Quil (currently focused surface, sid=201) ✓

## Gate Results

| Gate          | Status |
|---------------|--------|
| BUILD_GATE    | PASS   |
| SPAWN_GATE    | PASS   |
| SCHED_GATE    | PASS   |
| FAULT_GATE    | PASS   |
| SEXFILES_GATE | PASS   |
| CLOCK_GATE    | FAIL (pre-existing LAPIC) |

## Notes

- No new syscalls, no new PDX opcodes, no kernel/ABI changes.
- Key routing to Quil (not silk-shell's own handler) is because Quil has focus
  at proof time. Real user keys with shell focused would dispatch differently.
- silk-shell.key.route budget = 16; shell.key.ev_key.received budget = 4.
- Next phase: UINPUT_BUTTON_PROOF_V1 (real USB button events via /dev/uinput).
