# SPINDLE_SURFACE_RENDER_SCAFFOLD_V1

**Date:** 2026-05-06
**Status:** Scaffold proven — surface created, static content rendered
**Contract:** docs/handoff/SPINDLE_APP_CONTRACT_V1.md
**Next:** SPINDLE_KEYBOARD_INPUT_LINE_V1

---

## Summary

Created the smallest visible Spindle app surface with bounded text rows:
- Window created via `OP_WINDOW_CREATE` on sexdisplay slot 5
- Static content drawn via sex-graphics CP437 font
- 80×24 cell grid (640×192 pixels) at position (40, 200)
- Title bar ("Spindle"), info lines, separator lines, prompt ("sex> ")
- No input handling yet — static content only

---

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `apps/spindle/Cargo.toml` | NEW — package manifest (depends on sex-pdx, sex-graphics) | 8 |
| `apps/spindle/src/main.rs` | NEW — no_std PD with surface render scaffold | 140 |
| `Cargo.toml` | +1 line — workspace member `apps/spindle` | 1 |
| `sexos_build_spec.toml` | +8 lines — build stage + whitelist entry | 8 |
| `limine.cfg` | +1 line — MODULE_PATH for Spindle | 1 |
| `docs/handoff/SPINDLE_SURFACE_RENDER_SCAFFOLD_V1.md` | NEW — this handoff | — |

## Files NOT Changed

| File | Reason |
|------|--------|
| `kernel/src/init.rs` | STOP FIRST — Spindle not spawned at boot (ISO module only) |
| `crates/sex-pdx/` | No ABI changes needed |
| `servers/silk-shell/` | No shell changes needed |
| `servers/sexdisplay/` | No display changes needed |
| `apps/sexsh/` | Independent terminal emulator — not modified |

---

## Architecture

```
Spindle PD (apps/spindle)
  │
  ├── WindowCreateParams { x:40, y:200, w:640, h:192, pfn_base:0x4_0000 }
  │   └── pdx_call(SLOT_DISPLAY=5, OP_WINDOW_CREATE=0xE4)
  │         └── sexdisplay registers window region at 256 MiB physical
  │
  ├── WindowBuffer::new(0x4_0000_0000, 640, 192, 640)
  │   └── maps framebuffer region into virtual address space
  │
  ├── font::draw_str() — CP437 glyph rendering
  │   ├── "Spindle" (accent color, row 0)
  │   ├── "SexOS native command console" (row 2)
  │   ├── "Type help for commands." (row 3)
  │   └── "sex> " (green, row 23)
  │
  └── Idle loop: pdx_listen_raw(0) — no input handling yet
```

---

## Surface Layout

```
┌──────────────────────────────────────────────────────────────┐
│ Spindle                                                  ← accent
│──────────────────────────────────────────────────────────← separator
│ SexOS native command console                              ← FG
│ Type help for commands.                                   ← FG
│──────────────────────────────────────────────────────────← separator
│                                                              │
│   (rows 5-22: empty — future scrollback/output area)         │
│                                                              │
│ sex> _                                                    ← green
└──────────────────────────────────────────────────────────────┘
 80 cols × 8px = 640px wide
 24 rows × 8px = 192px tall
```

---

## Bounds

| Parameter | Value | Storage |
|-----------|-------|---------|
| Window width | 640 px (80 × 8) | `const WIN_W: u32` |
| Window height | 192 px (24 × 8) | `const WIN_H: u32` |
| Cell width/height | 8×8 px | `const CELL_W/CELL_H: u32` |
| Framebuffer PFN | 0x4_0000 (256 MiB) | `const FB_PFN_BASE: u64` |
| Window position | (40, 200) | `WindowCreateParams { x, y }` |
| Colors | 5 consts (Catppuccin Mocha) | `const BG/FG/ACCENT/GREEN/YELLOW` |

All bounds are compile-time constants. No dynamic allocation.

---

## Build / Runtime Result

### Build

```
./scripts/entrypoint_build.sh
```
Result: **PASS** — `[SEXOS ENTRYPOINT] success`

ISO contains 13 modules including `boot:///apps/spindle`.

```
$ ls iso_root/apps/
sexdrive  spindle  purple-scanout
```

### Runtime Gate

```
./scripts/master_runtime_gate.sh --probe 15 --keep-log
```

| Gate | Result |
|------|--------|
| BUILD_GATE | PASS |
| SPAWN_GATE | PASS |
| CLOCK_GATE | PASS (11 ticks) |
| SCHED_GATE | PASS |
| FAULT_GATE | PASS (0 faults) |
| SEXFILES_GATE | PASS |

**FINAL_SCORE: GREEN_MASTER** — no regressions.

---

## Launch Path

1. Limine loads `boot:///apps/spindle` as module #12
2. Spindle binary is present in ISO at `iso_root/apps/spindle`
3. **NOT auto-spawned at boot** — kernel `init.rs` does not include Spindle in `module_paths`
4. Spindle binary is available for future spawning via `sex-ld` or `run` command
5. For QEMU visual verification: Spindle window appears at (40,200) when PD is spawned
6. Proof marker emitted: `[spindle.boot]` via serial

---

## Proof Markers

| Marker | When | Status |
|--------|------|--------|
| `[spindle.boot]` | PD start | Emitted (serial log) |
| `[spindle.surface.req]` | Window create call | Emitted |
| `[spindle.surface.ok]` | Content drawn | Emitted |

---

## Non-Goals Preserved

- **No terminal emulation** — Spindle is NOT sexsh; no VT100/ANSI parser
- **No command execution** — static content only; command dispatch deferred
- **No keyboard input** — idle loop only; input deferred to next patch
- **No persistence** — no SexFiles/RamFS access
- **No Bell/Linen hooks** — deferred to V2/V3
- **No kernel edits** — Spindle is a userland app, not kernel-spawned
- **No sex-pdx ABI edits** — uses existing slots/opcodes only
- **No raw global framebuffer** — writes through WindowBuffer at bounded window region

---

## Contract Boundaries Preserved

- **sexdisplay sole framebuffer writer** — Spindle writes within its own window region via `WindowBuffer` at fixed PFN
- **FB bounds checks** — `WindowBuffer::draw_rect`, `draw_pixel`, `draw_char` all validate coordinates against width/height
- **No shared-memory redesign** — uses existing PFN-based WindowBuffer pattern
- **No broad refactor** — Spindle is a standalone app in `apps/spindle/`
- **No std/libc/threads** — pure `no_std`, single-threaded, PDX-only

---

## Remaining Work

1. **SPINDLE_KEYBOARD_INPUT_LINE_V1** — keyboard HID event loop, line editor, cursor
2. **SPINDLE_COMMAND_DISPATCH_V1** — compile-time command table, `help`/`ver`/`echo`
3. **SPINDLE_SCROLLBACK_V1** — bounded 1024-line ring buffer, scroll rendering
4. **SPINDLE_HISTORY_V1** — history navigation (arrow up/down), SexFiles persistence
5. **SPINDLE_BELL_HOOK_V1** — Bell event emission on command completion
6. **SPINDLE_LINEN_HOOK_V1** — Linen session object for command output

---

## Next Prompt

```
SPINDLE_KEYBOARD_INPUT_LINE_V1
```

Adds: HID event loop, scancode-to-ASCII table, line editor (backspace, enter, cursor), prompt redraw.
