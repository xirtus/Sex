# SPINDLE_TERMINAL_FINISH_PLAN_V1

**Date:** 2026-05-06
**Status:** Plan defined — 8 phases to full interactive terminal
**Current:** 14 handoffs, 11 source commits, PD 12 spawned, GREEN_MASTER

---

## Current Status

| Component | State |
|-----------|-------|
| Surface scaffold | 80×24 CP437 grid, static text, PFN 0x40000 |
| Line editor | 256-byte CmdLine, push/backspace/clear/redraw |
| Scrollback | 1024-line ring, 80-byte lines, scroll offset |
| Command dispatch | 20 commands, tokenizer, byte-match |
| History ring | 128 entries, in-memory only |
| Event ring | 32 entries, local only |
| Session identity | Local summary, Linen bridge pending |
| App launch | 4 targets, all honestly unavailable |
| Proof commands | 6 sub-commands, honest status |
| Kernel spawn | PD 12, Domain 12, PKU 12 |
| SexObject kind | `SpindleSession = 5`, `.spn` extension canon |
| Keyboard input | **STOP FIRST** (needs SLOT_SPINDLE = 14) |
| **Gates** | **GREEN_MASTER** (6/6, 0 faults) |

---

## Missing Pieces

| # | Piece | Blocker |
|---|-------|---------|
| 1 | Real keyboard input | sex-pdx: SLOT_SPINDLE (STOP FIRST) |
| 2 | Silk-shell HID routing | Depends on #1 |
| 3 | Visible surface (silk-shell) | Silk-shell frame/tab + sexdisplay upsert |
| 4 | SexFiles history persistence | RamFS PDX calls (post #1) |
| 5 | Bell event bridge | OP_BELL_NOTIFY (post #1) |
| 6 | Linen session object | `.spn` file in Linen browser (post #1) |
| 7 | Command set finalization | Tab completion, color, UTF-8 |

---

## Phase Plan

### Phase 1: sex-pdx Slot (STOP FIRST)
**Prompt:** `SPINDLE_SEXPDX_SLOT_V1`

| File | Change |
|------|--------|
| `crates/sex-pdx/src/lib.rs` | +1: `pub const SLOT_SPINDLE: u64 = 14;` |

- Smallest possible change (1 line)
- Follows existing pattern (SLOT_QUIL=11, SLOT_BELL=12, SLOT_LINEN=13)
- No kernel edits, no scheduler changes
- Build: PASS, Gate: GREEN_MASTER

### Phase 2: Silk-Shell Surface + HID Routing
**Prompt:** `SPINDLE_SILKSHELL_ROUTE_V1`

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | +15 lines |

Changes:
1. `SURFACE_ID_SPINDLE: u64 = 400` (app range)
2. Frame/tab creation in boot init
3. Lifecycle registration: `lifecycle_register(SURFACE_ID_SPINDLE, LifecycleState::Visible)`
4. HID forwarding: `if FOCUSED_SURFACE_ID == SURFACE_ID_SPINDLE { pdx_call(SLOT_SPINDLE, OP_HID_EVENT, ...) }`
5. Sexdisplay upsert for surface visibility
6. Key route budget marker: `[silk-shell.keyboard.forward.spindle]`

### Phase 3: Spindle Real Keyboard Input
**Prompt:** `SPINDLE_REAL_KEYBOARD_V1`

| File | Change |
|------|--------|
| `apps/spindle/src/main.rs` | +15 lines |

Changes:
1. Replace idle loop with HID event loop
2. `handle_key()`: scancode→ASCII table, route to CmdLine
3. Enter → dispatch → scrollback → redraw
4. Markers: `[spindle.input.recv]`, `[spindle.line.append]`, etc.

### Phase 4: SexFiles History Persistence
**Prompt:** `SPINDLE_SEXFILES_PERSIST_V1`

| File | Change |
|------|--------|
| `apps/spindle/src/main.rs` | +30 lines |

Changes:
1. On boot: `pdx_call(SLOT_STORAGE, OP_RAMFS_OPEN, "spindle_history", ...)` — read saved history
2. On Enter: `pdx_call(SLOT_STORAGE, OP_RAMFS_WRITE, handle, ...)` — append command
3. On close: save history to RamFS file
4. Graceful fallback: if SexFiles unavailable, operate memory-only

### Phase 5: Bell Event Bridge
**Prompt:** `SPINDLE_BELL_BRIDGE_V1`

| File | Change |
|------|--------|
| `apps/spindle/src/main.rs` | +10 lines |

Changes:
1. On command success: `pdx_call(SLOT_BELL, OP_BELL_NOTIFY, ...)` with CmdOk event
2. On command failure: CmdFail event
3. On unknown command: CmdUnknown event
4. Replace local EventRing with real Bell delivery

### Phase 6: Linen Session Object (.spn)
**Prompt:** `SPINDLE_LINEN_SPN_V1`

| File | Change |
|------|--------|
| `apps/spindle/src/main.rs` | +10 lines |
| `servers/linen/src/main.rs` | +5 lines (SpindleSession kind mapping) |

Changes:
1. On boot: create `.spn` session object in Linen via PDX
2. `session` command shows real Linen object ID
3. Linen browser shows Spindle session as `.spn` file
4. SexObjectRef uses global SexFiles object_id

### Phase 7: Surface/Display Integration
**Prompt:** `SPINDLE_DISPLAY_SURFACE_V1`

| File | Change |
|------|--------|
| `apps/spindle/src/main.rs` | +20 lines |

Changes:
1. Remove direct PFN framebuffer access
2. Use silk-shell surface buffer (zero-copy or PDX-mediated)
3. `pdx_call(SLOT_SHELL, OP_APP_SURFACE_REQ, 400, ...)` for surface
4. Render via silk-shell → sexdisplay pipeline
5. sexdisplay remains sole FB writer

### Phase 8: Command Set Finalization
**Prompt:** `SPINDLE_COMMAND_SET_V1`

| File | Change |
|------|--------|
| `apps/spindle/src/main.rs` | +50 lines |

Changes:
1. Tab completion (simplest: match prefix against command table)
2. Arrow key history navigation (up/down through History ring)
3. Command argument parsing improvements
4. Color output (CP437 palette codes)
5. UTF-8 support (multi-byte sequences in CmdLine)
6. Help text for all commands
7. Version bump: V1.0.0

---

## STOP FIRST Triggers

| Phase | Trigger | Why |
|-------|---------|-----|
| 1 | sex-pdx: SLOT_SPINDLE | ABI change (slot registry) |
| 2-8 | None (approved after Phase 1) | All within existing patterns |

---

## Proof Markers Per Phase

| Phase | New Markers |
|-------|-------------|
| 1 | (none — constant definition only) |
| 2 | `[silk-shell.keyboard.forward.spindle]`, `[shell.spindle.surface]` |
| 3 | `[spindle.input.recv]`, `[spindle.line.append]`, `[spindle.line.backspace]`, `[spindle.line.enter]` |
| 4 | `[spindle.history.save]`, `[spindle.history.load]`, `[spindle.history.persist]` |
| 5 | `[spindle.bell.notify]`, `[spindle.bell.cmd_ok]`, `[spindle.bell.cmd_fail]` |
| 6 | `[spindle.linen.object]`, `[spindle.linen.spn]` |
| 7 | `[spindle.surface.route]`, `[spindle.surface.shell]` |
| 8 | `[spindle.cmd.complete]`, `[spindle.cmd.history_nav]` |

---

## No_Std Constraints (All Phases)

| Constraint | Enforcement |
|-----------|-------------|
| No `use std::` | DummyAllocator (no real heap) |
| No `extern crate libc` | Pure PDX-only IPC |
| No `Vec`/`String` growth | All buffers fixed-size arrays |
| No `fork`/`exec`/spawn | Commands are local Rust functions |
| No PTY/ioctl/termios | Spindle is NOT a terminal emulator |
| No `/bin/sh` or shell | Tokenizer is whitespace-only |
| No host command execution | dispatch() is pure match on bytes |

---

## Display Ownership Boundaries

| Component | Owns | Spindle Accesses Via |
|-----------|------|---------------------|
| sexdisplay | Final framebuffer pixels | Silk-shell → sexdisplay (never direct) |
| silk-shell | Surface geometry, frames, tabs, focus | `OP_APP_SURFACE_REQ`, PDX surface buffer |
| Spindle | Text content, scrollback, cursor position | Render to surface buffer only |
| sex-graphics | CP437 font glyphs | `font::draw_str`, `font::draw_char` |

Spindle NEVER writes raw framebuffer after Phase 7.

---

## Input Ownership Boundaries

| Component | Owns | Route |
|-----------|------|-------|
| sexinput | HID normalization, scancode→event | → silk-shell via PDX |
| silk-shell | Focus policy, key routing, shortcuts | → Spindle via `SLOT_SPINDLE` |
| Spindle | Line editor, history nav, dispatch | ← silk-shell via `pdx_listen_raw(0)` |

Spindle NEVER reads raw HID. Focus isolation: unfocused Spindle receives zero keyboard events.

---

## Persistence / SexObject Path

```
Spindle Enter
  └→ hist.push(cmd)
  └→ pdx_call(SLOT_STORAGE, OP_RAMFS_WRITE, "spindle_history", cmd)

Spindle Boot
  └→ pdx_call(SLOT_STORAGE, OP_RAMFS_OPEN, "spindle_history")
  └→ pdx_call(SLOT_STORAGE, OP_RAMFS_READ, handle, buf)
  └→ hist.ring populated from saved data

Linen .spn Visibility
  └→ pdx_call(SLOT_LINEN, OP_LINEN_CREATE_OBJECT, "Spindle", SpindleSession)
  └→ Linen browser shows Spindle.spn
  └→ SexObjectRef { object_id: sexfiles_global_id, generation }
```

---

## Minimal Command Set (Final)

| # | Command | Phase | Status |
|---|---------|-------|--------|
| 1 | `help` | Done | Implemented |
| 2 | `clear` | Done | Implemented |
| 3 | `status` | Done | Implemented |
| 4 | `pd` | Done | Implemented |
| 5 | `servers` | Done | Implemented |
| 6 | `apps` | Done | Implemented |
| 7 | `launch <app>` | 2 | Needs surface routing |
| 8 | `history` | 4 | Needs SexFiles persistence |
| 9 | `history clear` | Done | Implemented |
| 10 | `events` | 5 | Needs Bell bridge |
| 11 | `events clear` | 5 | Needs Bell bridge |
| 12 | `session` | 6 | Needs Linen .spn |
| 13 | `proof` | Done | Implemented |
| 14 | `faults` | Done | Implemented |
| 15 | `close` | Done | Implemented |
| 16 | `bell` | 5 | Needs Bell bridge |
| 17 | `files` | 4 | Needs SexFiles bridge |

---

## First Implementation Prompt

```
SPINDLE_SEXPDX_SLOT_V1
```

Single change: add `SLOT_SPINDLE = 14` to `crates/sex-pdx/src/lib.rs`. This unblocks Phases 2-8.

---

## Handoff Path

| # | Handoff |
|---|---------|
| 1 | `SPINDLE_SEXPDX_SLOT_V1.md` |
| 2 | `SPINDLE_SILKSHELL_ROUTE_V1.md` |
| 3 | `SPINDLE_REAL_KEYBOARD_V1.md` |
| 4 | `SPINDLE_SEXFILES_PERSIST_V1.md` |
| 5 | `SPINDLE_BELL_BRIDGE_V1.md` |
| 6 | `SPINDLE_LINEN_SPN_V1.md` |
| 7 | `SPINDLE_DISPLAY_SURFACE_V1.md` |
| 8 | `SPINDLE_COMMAND_SET_V1.md` |
| 9 | `SPINDLE_TERMINAL_COMPLETE_AUDIT.md` |
