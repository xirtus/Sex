# SPINDLE_REAL_KEYBOARD_V1

**Date:** 2026-05-06
**Status:** HID event loop proven — receives keys via SLOT_SPINDLE, processes into line editor
**Previous:** SPINDLE_SILKSHELL_ROUTE_V1
**Next:** SPINDLE_SEXFILES_PERSIST_V1 (Phase 4)

---

## Summary

Replaced Spindle's idle loop with a real HID event loop:
- Listens for `OP_HID_EVENT` (0x202) messages via `pdx_listen_raw(0)`
- Scancode-to-ASCII table (US QWERTY set 1, matches sexsh)
- Printable keys → `CmdLine.push()` → `[spindle.line.append]`
- Backspace (0x0E) → `CmdLine.backspace()` → `[spindle.line.backspace]`
- Enter (0x1C) → `[spindle.line.enter]`
- Escape (0x01) → `CmdLine.clear()`
- Unknown scancodes silently ignored
- All operations bounded — no overflow, no heap growth

---

## Input Route

```
User keystroke
  → sexinput (HID normalization)
    → silk-shell (focus/routing)
      → if FOCUSED_SURFACE_ID == SURFACE_ID_SPINDLE (0x99):
          pdx_call(SLOT_SPINDLE, OP_HID_EVENT, scancode, value=1, EV_KEY)
            → Spindle PD 12 (pdx_listen_raw(0))
              → handle_key(scancode, &mut line)
                → CmdLine::push() / backspace() / clear()
                → serial_println markers
```

---

## Scancode Table (Set 1, US QWERTY)

| Range | Keys |
|-------|------|
| 0x02-0x0B | 1 2 3 4 5 6 7 8 9 0 |
| 0x10-0x19 | q w e r t y u i o p |
| 0x1E-0x26 | a s d f g h j k l |
| 0x2C-0x32 | z x c v b n m |
| 0x39 | Space |
| 0x0C/0x0D | - / = |
| 0x1A/0x1B | [ / ] |
| 0x27/0x28 | ; / ' |
| 0x29/0x2B | ` / \ |
| 0x33-0x35 | , . / |
| 0x0F | Tab |
| 0x1C | **Enter** |
| 0x0E | **Backspace** |
| 0x01 | **Escape** (clear) |

---

## Expected Markers (When Spindle Focused)

| Marker | Trigger |
|--------|---------|
| `[spindle.input.recv]` | Every key event received |
| `[spindle.line.append]` | Printable key appended |
| `[spindle.line.backspace]` | Backspace |
| `[spindle.line.enter]` | Enter |

---

## Files Changed

| File | Change |
|------|--------|
| `apps/spindle/src/main.rs` | +60 lines — HID loop, handle_key, scancode table |
| `docs/handoff/SPINDLE_REAL_KEYBOARD_V1.md` | NEW |

## Files NOT Changed

| File | Reason |
|------|--------|
| `servers/silk-shell/` | Routing already in Phase 2 |
| `crates/sex-pdx/` | SLOT_SPINDLE already in Phase 1 |
| `kernel/src/` | No kernel changes |
| `servers/sexinput/` | Input normalization unchanged |

---

## Build / Runtime Result

| Check | Result |
|-------|--------|
| cargo check | PASS (1 warning: unused variable) |
| entrypoint_build.sh | PASS |
| master_runtime_gate | **GREEN_MASTER** (6/6) |
| Faults | **0** |

### Serial Log

```
[silk-shell.spindle.route.ready] slot=14 surface=153
[kernel.spawn.spindle] id=12
[spindle.boot]
[spindle.ready]
```

Markers `[spindle.input.recv]`, `[spindle.line.append]`, etc. appear when Spindle surface (0x99) is focused and keys are pressed.

---

## Next Prompt

```
SPINDLE_SEXFILES_PERSIST_V1
```

Phase 4: SexFiles RamFS history persistence.
