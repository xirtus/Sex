# SPINDLE_APP_CONTRACT_V1

**Date:** 2026-05-06
**Status:** Contract defined — no implementation yet
**Scope:** docs-only — app surface identity, bounds, ownership, non-goals
**Next:** SPINDLE_SURFACE_RENDER_SCAFFOLD_V1

---

## 1. App Identity

| Field | Value |
|-------|-------|
| **Name** | Spindle |
| **Kind** | Native SexOS command console / developer terminal |
| **SexObjectKind** | `SexObjectKind::SpindleSession = 5` (defined in `crates/sex-object-model/src/lib.rs`) |
| **App surface ID** | `400` (app range; silk-shell validates `surface_id >= 200`) |
| **App path** | `apps/spindle/` — follows existing `apps/sex-edit`, `apps/sex-calc`, `apps/sexsh` convention |
| **Launch surface title** | `"Spindle"` (7 bytes, fits 24-byte RamFS name limit) |
| **PD slot assignment** | PD 12 or next available (kernel init allocates) |

---

## 2. What Spindle IS

Spindle is the native SexOS command console. It is a userland app PD that:
- Renders a text surface via silk-shell → sexdisplay (zero direct FB)
- Reads keyboard HID events routed through shell's focus/input policy
- Accepts typed commands in a bounded line editor
- Dispatches to a fixed set of compile-time registered native commands
- Maintains bounded scrollback output and command history
- Persists history to SexFiles RamFS (V2)
- Emits structured reply lines — no escape codes

### Spindle vs sexsh vs cosmic-term

| Feature | Spindle (this contract) | sexsh (existing) | cosmic-term (placeholder) |
|---------|------------------------|------------------|---------------------------|
| Terminal emulation | **NO** | VT100/ANSI | Unknown |
| Escape code parser | **NO** | Yes (state machine) | Unknown |
| Native command dispatch | **YES** | Partial (built-in set) | Unknown |
| Shell scripting | **NO** | No | Unknown |
| PTY / pseudoterminal | **NO** | No | Unknown |
| Scrollback | Bounded ring buffer (1024) | Cell grid | Unknown |
| Font | sex-graphics CP437 | sex-graphics CP437 | Unknown |
| Framebuffer access | **NEVER** — shell/display only | Zero-copy PDX surface | Unknown |

---

## 3. Non-Goals (Explicitly EXCLUDED)

| Non-Goal | Reason |
|----------|--------|
| POSIX shell (`/bin/sh`, `bash`, `zsh`) | SexOS has no POSIX. No fork/exec. Spindle is not a shell. |
| PTY / pseudoterminal | No tty layer. No pty/tty pair. No ioctl. |
| Shell escaping / quoting | No shell language. Commands are discrete typed lines. |
| Pipes (`\|`), redirects (`>`, `<`) | No process composition. Each command runs atomically. |
| Environment variables (`$PATH`, `$HOME`) | No process model. Bounded key=value table (future V3). |
| Job control (`&`, `fg`, `bg`, `jobs`) | No background processes. Single-threaded event loop. |
| ANSI/VT100 escape sequences | Sexsh handles terminal emulation. Spindle is plain text. |
| Color / styling (V1) | Deferred. V1 is monochrome text on app surface. |
| Host command execution (`exec`, `system`) | No host OS. Every command is a Spindle-internal handler. |
| Dynamic command loading | All commands are compile-time registered. No dlopen. |
| Tab completion (V1) | Deferred to V2. |
| Syntax highlighting (V1) | Deferred to V2. |
| Unbounded output | All output lines ≤ 80 chars. Scrollback ring is bounded. |
| Copy/paste (V1) | Deferred. Requires shell-level clipboard service. |

---

## 4. V1 Responsibilities

### 4.1 Launchable App Surface

Spindle requests one surface from silk-shell via the established app surface contract:

```
pdx_call(SLOT_SHELL, OP_APP_SURFACE_REQ, surface_id=400, title_id=SPINDLE_TITLE_ID, reserved=0)
```

- Silk-shell validates: surface_id >= 200, title_id != 0, not already registered
- On accept: shell creates ShellFrame + ShellTab, upserts on sexdisplay
- Shell routes keyboard HID events to Spindle when it has focus
- Spindle receives input events on its PDX listen loop (slot 0)

### 4.2 Keyboard Line Input

Spindle maintains a bounded line editor:

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Max command bytes | 256 | Fits in 8 PDX arg registers (8 × 8 bytes = 64 bytes per register set, multi-call works) |
| Cursor position | 0..len | Single insertion point, no selection |
| Backspace | Delete char before cursor | Standard line editing |
| Left/Right arrows | Move cursor | Within 0..len |
| Home/End | Jump to start/end | Standard |
| Enter | Execute command, push to history, clear buffer | Atomic dispatch |
| Escape | Clear buffer | Discard current input |

**No multi-line input in V1.** Each command is one line.

### 4.3 Bounded Scrollback

| Parameter | Value | Rationale |
|-----------|-------|-----------|
| Max visible rows | 24 | Classic terminal height |
| Max visible cols | 80 | Classic terminal width |
| Max scrollback lines | 1024 | Bounded ring buffer; ~48 screens of history |
| Scrollback storage | Fixed-size ring buffer `[ScrollLine; 1024]` | No heap growth, no Vec |
| Per-line max chars | 80 | Bounded to visible width |
| Line wrapping | Hard wrap at 80 chars | No soft-wrap; no reflow |

Each scrollback line is a fixed 80-byte `[u8; 80]` buffer — zero allocation per line.
Ring buffer wraps: oldest line overwritten when full.

### 4.4 Bounded Native Command Dispatcher

Commands are registered at compile time in a const table:

```rust
// Conceptual (not implemented)
struct CommandEntry {
    name: [u8; 16],      // command name (≤ 16 bytes)
    min_args: u8,         // minimum argument count
    max_args: u8,         // maximum argument count
    handler: fn(&[&[u8]]) -> Result<(), i64>,  // command handler
    help: &'static [u8],  // help text (≤ 80 bytes)
}
```

| Parameter | Value |
|-----------|-------|
| Max commands | 16 (compile-time bound) |
| Command name max | 16 bytes |
| Args per command max | 4 |
| Arg max bytes | 64 per arg |
| Dispatch time | < 1ms (no I/O in handler unless explicit PDX call) |
| Unknown command | `"unknown command: <name>"` output line |
| Help text | `"help [cmd]"` — prints all commands or one detail |

### 4.5 Command Set (V1 Proposals)

These are compile-time registered native handlers. None execute external binaries. None fork processes.

| # | Command | Args | Description |
|---|---------|------|-------------|
| 1 | `help` | `[cmd]` | List commands or detail one |
| 2 | `clear` | — | Clear visible area (scrollback preserved) |
| 3 | `echo` | `<text>` | Print text to output |
| 4 | `ver` | — | Print Spindle version + SexOS build info |
| 5 | `pd` | `[id]` | List running PDs or stat one |
| 6 | `mem` | — | Show allocator stats (total/free/used) |
| 7 | `time` | — | Show system uptime (ticks) |
| 8 | `store` | `<key> [val]` | Get/set sexstore KV (delegates to sexstore PDX) |
| 9 | `ls` | `[name]` | List files in RamFS or stat one (delegates to sexfiles PDX) |
| 10 | `cat` | `<name>` | Read file content (delegates to sexfiles PDX) |
| 11 | `run` | `<name>` | Spawn PD by module name (delegates to kernel/sex-ld) |
| 12 | `bell` | `<msg>` | Send Bell event (delegates to sexbell PDX) |
| 13 | `mesh` | `<query>` | Query Mesh fact graph (delegates to mesh PDX) |
| 14 | `linen` | `<name>` | Open Linen session (delegates to linen PDX) |
| 15 | `quil` | `<name>` | Open/edit Quil document (delegates to quil PDX) |
| 16 | `exit` | — | Close Spindle (signal shell to tombstone surface) |

All delegation uses PDX calls — Spindle never accesses another server's memory.

### 4.6 Command History (V2: SexFiles Persistence)

| Parameter | Value |
|-----------|-------|
| Max history entries | 128 |
| History storage | Fixed-size ring `[HistoryEntry; 128]` |
| History entry | `{ cmd: [u8; 256], timestamp: u64 }` |
| Persistence | Deferred to V2: save/load from SexFiles RamFS |
| RamFS file name | `"spindle_history"` (15 bytes) |
| Arrow up/down | Navigate history in-place in command buffer |

### 4.7 Bell/Linen Hooks (Deferred)

- **Bell (V2):** Spindle emits Bell events on command completion/failure. SilkBar displays notifications.
- **Linen (V3):** Each Spindle command output can be saved as a Linen object (persistent session record).
- Neither hook exists in V1 — V1 is a standalone command console.

---

## 5. Bounds Summary

All storage is fixed-size. No dynamic allocation after init. No `Vec::push`. No unbounded growth.

| Resource | Type | Max | Bytes |
|----------|------|-----|-------|
| Command buffer | `[u8; 256]` | 1 | 256 |
| Scrollback ring | `[[u8; 80]; 1024]` | 1024 lines | 81,920 |
| Command table | `[CommandEntry; 16]` | 16 entries | ~512 |
| History ring | `[HistoryEntry; 128]` | 128 entries | ~33,024 |
| Output line buffer | `[u8; 80]` | 1 (temp) | 80 |
| PDX reply buffer | `u64` (register) | 1 | 8 |
| Surface geometry | 800 × 288 (80×24 @ 8×8 glyph) | 1 | — |

**Total static memory: ~116 KB** — fits comfortably in a single 4K page frame (with spill to allocator heap for scrollback ring).

---

## 6. Ownership Map

| Component | Owner | Spindle's Role |
|-----------|-------|---------------|
| **Command buffer** | Spindle | Owns, edits, clears |
| **Line editor** | Spindle | Owns cursor, insertion, deletion |
| **Scrollback buffer** | Spindle | Owns ring buffer, push, render |
| **Command dispatch table** | Spindle | Owns compile-time table |
| **History ring** | Spindle | Owns in-memory ring; delegates persistence to sexfiles |
| **App surface lifecycle** | silk-shell | Spindle calls OP_APP_SURFACE_REQ; shell owns frame/tab/focus |
| **Input routing** | silk-shell | Shell routes HID events to focused surface |
| **Final pixels** | sexdisplay | Spindle NEVER writes framebuffer |
| **Persistence (history)** | sexfiles | Spindle delegates save/load via RamFS PDX calls |
| **Notification events** | sexbell (V2) | Spindle emits events; Bell delivers |
| **Session metadata** | linen (V3) | Spindle stores session records via Linen |
| **Raw HID events** | sexinput | Shell reads HID; Spindle reads shell-routed events |

---

## 7. STOP FIRST Rules

The following are **explicitly prohibited** without STOP FIRST handoff approval:

| # | Prohibited Action | Why |
|---|-------------------|-----|
| 1 | Kernel edits (`kernel/src/`) | No kernel changes for Spindle. Spindle is a userland app. |
| 2 | sex-pdx ABI edits (`crates/sex-pdx/`) | No new slots or opcodes. Spindle uses existing slots (SHELL=6, STORAGE=1, etc.) |
| 3 | Raw framebuffer writes from Spindle | sexdisplay is sole FB writer. Spindle renders via shell → sexdisplay. |
| 4 | Shared backing buffer redesign | No shared memory between Spindle and shell/display. |
| 5 | POSIX/PTY emulation | Spindle is a native command console. No pseudoterminal. No ioctl. |
| 6 | Unbounded heap/Vec/String growth | All buffers are fixed-size arrays. No dynamic allocation after init. |
| 7 | `use std::` or `extern crate libc` | Strict no_std. Spindle depends on sex-pdx only. |
| 8 | Direct input device access | Input flows through sexinput → shell → Spindle. No raw HID from Spindle. |
| 9 | Direct SexFiles block device access | Spindle uses RamFS via PDX calls. No raw disk. No superblock access. |
| 10 | External process spawning (`fork`/`exec`) | Not possible. No POSIX. `run` command delegates to sex-ld via PDX. |

---

## 8. PDX Slot Usage

Spindle uses ONLY existing PDX slots. No new slot allocation.

| Slot | Service | Used For |
|------|---------|----------|
| 0 | Self ring | Listen for input events, command results |
| 6 | `SLOT_SHELL` | App surface request (OP_APP_SURFACE_REQ = 0xFA) |
| 1 | `SLOT_STORAGE` | SexFiles RamFS calls (history save/load, ls, cat) |
| ? | `SLOT_SEXSTORE` | KV get/set (store command) |
| ? | `SLOT_BELL` | Bell event emit (future V2) |
| ? | `SLOT_LINEN` | Linen session open (future V3) |
| ? | `SLOT_QUIL` | Quil document open (future V3) |

---

## 9. Rendering Model

Spindle renders text through the existing silk-shell → sexdisplay pipeline:

1. Spindle formats a line of text into an 80-byte output buffer
2. Spindle sends the line to its surface via PDX (exact opcode TBD — matches existing app surface draw protocol)
3. Silk-shell receives the draw command and routes to sexdisplay
4. Sexdisplay renders CP437 glyphs at the surface's cell grid positions
5. Spindle NEVER directly writes framebuffer memory

**V1 rendering:**
- Monochrome text (white on dark background, or shell-default theme)
- Fixed 8×8 CP437 glyph grid (matching sexsh font)
- Surface geometry: 800 × 288 pixels (80 cols × 8px, 24 rows × 8px + header)
- Top row reserved for title bar (shell-managed)
- No cursor blink in V1 (solid block cursor)

---

## 10. Proof Gate (Minimal V1)

Spindle V1 boots as a stub PD proving:
- App compiles as `no_std` SexOS binary
- App can request a surface from silk-shell
- App receives keyboard HID events when focused
- App echos typed characters to its surface
- App responds to `help` and `ver` commands with structured output

Proof gate: `SEXOS_SPINDLE_PROOF=1` (compile-time `option_env!`)

Proof markers (proposed):
```
[spindle.boot]          — PD boots, allocator initialized
[spindle.surface.req]   — Surface request to shell (400, "Spindle")
[spindle.surface.ok]    — Surface accepted by shell
[spindle.input.echo]    — HID event received, character echoed
[spindle.cmd.help]      — "help" command returns command list
[spindle.cmd.ver]       — "ver" command returns version info
[spindle.proof.done]    — All proof stages pass
```

---

## 11. Files Proposed (Future Implementation)

| File | Purpose |
|------|---------|
| `apps/spindle/Cargo.toml` | Package manifest (depends on sex-pdx) |
| `apps/spindle/src/main.rs` | no_std PD stub, surface request, event loop |
| `apps/spindle/src/editor.rs` | Line editor (cursor, insert, delete, history nav) |
| `apps/spindle/src/commands.rs` | Compile-time command table + handlers |
| `apps/spindle/src/scrollback.rs` | Bounded ring buffer for scrollback lines |
| `Cargo.toml` | Add `"apps/spindle"` to workspace members |
| `sexos_build_spec.toml` | Add `build_spindle` stage |
| `limine.cfg` | Add `MODULE_PATH=boot:///apps/spindle` |
| `docs/handoff/SPINDLE_APP_CONTRACT_V1.md` | This document |

**NOT changed:**
- `kernel/src/` — no kernel edits (PD spawn deferred to sex-ld or init.rs with approval)
- `crates/sex-pdx/` — no ABI changes
- `servers/silk-shell/` — no shell changes (existing app surface contract suffices)
- `servers/sexdisplay/` — no display changes

---

## 12. Comparison with Existing Apps

| App | Location | Status | Relationship to Spindle |
|-----|----------|--------|------------------------|
| `sexsh` | `apps/sexsh/` | VT100 terminal | Spindle is NOT a terminal emulator. Sexsh handles escape codes; Spindle is plain native commands. |
| `cosmic-term` | `apps/cosmic-term/` | Placeholder | Unknown scope. Likely a COSMIC terminal port. Spindle is SexOS-native. |
| `sex-calc` | `apps/sex-calc/` | Calculator app | Independent. May be invocable from Spindle via `run` command. |
| `sex-edit` | `apps/sex-edit/` | Text editor | Independent. Spindle's line editor is single-line; sex-edit is a full editor. |
| `sex-files` | `apps/sex-files/` | File manager | Independent. Spindle's `ls`/`cat` commands provide CLI access. |

---

## 13. Exact Next Prompt

```
SPINDLE_SURFACE_RENDER_SCAFFOLD_V1
```

Creates:
- `apps/spindle/` as a no_std PD stub
- Proves surface request + echo loop
- No command dispatch yet
- No scrollback yet
- Proof markers: boot, surface.req, surface.ok, input.echo

---

## Contract Boundaries Preserved

- **No Linux/POSIX assumptions** — Spindle is a native SexOS console with no shell/PTY/ioctl
- **No std/libc/threads** — pure no_std Rust, single-threaded event loop
- **MPK/PKU/PKEY isolation preserved** — Spindle runs in its own PD, isolated from shell/display/storage
- **sexdisplay sole framebuffer writer** — Spindle never writes FB; renders through shell pipeline
- **FB bounds checks preserved** — no FB access means no risk of violation
- **No shared-memory redesign** — all inter-PD communication via PDX registers
- **No kernel edits** — Spindle is a userland app only
- **No sex-pdx ABI edits** — uses existing slots and opcodes
- **No broad refactor** — Spindle is an additive app, like sexsh/sex-edit/sex-calc
- **Bounded storage** — all buffers fixed-size; no Vec growth; no unbounded heap
