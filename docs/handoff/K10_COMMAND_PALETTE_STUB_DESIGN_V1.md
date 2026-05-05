# K10: Command Palette Stub Design

**Status:** Handoff (design only — no code)
**Date:** 2026-05-05
**Purpose:** Define the command palette as a shell-owned action router, not a new
authority system, app, or renderer policy path. This document scopes K11 implementation
to avoid architectural drift.

## Verdict

```
╔══════════════════════════════════════════════════════════════╗
║                    SAFE_TO_STUB                              ║
╠══════════════════════════════════════════════════════════════╣
║ Command palette is shell-owned action routing only.          ║
║ No new authority, no new display primitives, no sexdisplay   ║
║ changes, no kernel/ABI edits, no PDX opcodes.                ║
║ Follows existing I1-I3 placeholder surface pattern.          ║
╚══════════════════════════════════════════════════════════════╝
```

## Command Palette Role

The command palette is a **shell-owned action selector** that:

- Lives entirely in silk-shell (PKEY 3, no cross-PD communication)
- Lists available shell actions (commands) for the user to select and execute
- Routes selected commands through **existing** `SurfaceAction` dispatch paths
- Does **not** own authority — Collar gates (J5) still apply to gated operations
- Does **not** own rendering policy — uses existing 0xEF fill rect only
- Does **not** bypass focus/lifecycle — executed commands still pass through
  `try_set_focus()`, lifecycle FSM guards, and focus gates

## Non-Goals (Explicit)

| Non-Goal | Reason |
|----------|--------|
| Fuzzy search / text input | Requires text rendering → sexdisplay change (STOP FIRST) |
| App-provided command manifests | Cross-PD protocol → new PDX opcodes (STOP FIRST) |
| Command history / persistence | Storage → filesystem code (STOP FIRST) |
| Keyboard-driven text filter | Text input → editor subsystem (STOP FIRST) |
| Real-time command discovery | Requires active command registry → lifecycle change |
| Replace J/K/PrintScreen keyboard shortcuts | Shortcuts remain primary; palette is alternative |
| Cross-PD command execution | Each PD owns its actions; palette only routes shell-local |

## State Model

```
┌──────────────────────────────────────────────┐
│              Command Palette                  │
│                                              │
│  Surface: overlay (0x98, frame-owned)         │
│  State:   palette_open: bool                  │
│           palette_selected_index: u8          │
│  Data:    COMMAND_LIST: [CommandDef; 5]       │
│           (static const, no heap)             │
│  Render:  0xEF header + proof-marker rows     │
│           (mirrors J2/K3 pattern)             │
└──────────────────────────────────────────────┘
```

All state is shell-local static data. No heap. No PDX. No filesystem.

## Command Enum Proposal

```rust
/// Shell commands exposed via the command palette.
/// Each command routes to an existing SurfaceAction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Command {
    /// Open selected Linen object in Quil (existing PrintScreen action)
    OpenSelectedInQuil = 0,
    /// Toggle focus to Linen surface (existing ToggleLinen)
    FocusLinen = 1,
    /// Toggle focus to Quil surface (existing ToggleQuil)
    FocusQuil = 2,
    /// Switch to next scene (existing AccessSceneNext)
    SceneNext = 3,
    /// Open Atlas overview (existing ToggleAtlas)
    OpenAtlas = 4,
}
```

### Command-to-Action Routing

| Command | SurfaceAction | Gate Dependency |
|---------|--------------|----------------|
| OpenSelectedInQuil | `OpenObjectInQuil` | `FOCUSED_SURFACE_ID == SURFACE_ID_LINEN` (K9) |
| FocusLinen | `ToggleLinen` | None (toggle always works) |
| FocusQuil | `ToggleQuil` | None (toggle always works) |
| SceneNext | `AccessSceneNext` | Scene exists check |
| OpenAtlas | `ToggleAtlas` | None (toggle always works) |

The palette command dispatches to the **same** `SurfaceAction` handler that keyboard
shortcuts use. Zero execution path divergence. Collar gates (J5), Bell events (J7),
and Mesh diagnostics (J6) fire identically whether triggered via keyboard or palette.

### V1 Constraint: 5 Commands Max

First 5 commands are static. Future commands can be added as const array entries
without changing the pattern. No dynamic registration.

## Trigger Proposal

**Key: Scancode 0x29** (backtick/tilde `` ` ``)

- Common convention for command palettes across many systems
- Unused in current `scancode_to_action()` table
- Single key, no modifier required
- Toggles palette open/close (same as F7 for scene settings, F10 for Atlas)

**Alternative:** 0x33 (comma `,`) or 0x34 (period `.`) — also unused, but less conventional.

Recommendation: **0x29 (backtick)** for discoverability.

## Render Proposal (Within Current Constraints)

Follows the established J2/K3 pattern exactly:

```
Surface 0x98 "Command Palette"
┌──────────────────────────────────────┐
│ 0xEF header bar (accent color)       │  ← 28px tall, palette accent
│                                      │
│ Proof-marker rows (one per command): │  ← no visual rows
│   [palette.row] index=0 cmd=0        │
│     name=OpenSelectedInQuil          │
│     selected=true                    │
│   [palette.row] index=1 cmd=1        │
│     name=FocusLinen                  │
│     selected=false                   │
│   ...                                │
│                                      │
│ Geometry: 300w × 200h, centered      │  ← small fixed overlay
└──────────────────────────────────────┘
```

**One 0xEF fill rect total.** Header only. Rows are proof-marker-only, matching the
constraint that sexdisplay supports exactly one fill rect per surface. Visual row
highlighting requires multi-rect display support (STOP FIRST).

## Execution Path for OpenSelectedInQuil

When the palette command "OpenSelectedInQuil" is executed:

```
palette execute → SurfaceAction::OpenObjectInQuil
  → K9 gate: FOCUSED_SURFACE_ID == SURFACE_ID_LINEN?
    → NO:  [palette.cmd.reject] reason=not_focused  (reject marker)
    → YES: open_linen_object_in_quil(linen_selected_object_id())
            → [collar.gate.check] / [collar.gate.allow_stub]  (J5)
            → [linen.quil.open.dynamic_id]                     (J4)
            → [linen.quil.buffer.linked]                       (J4)
            → [mesh.object_link.row]                           (J6)
            → [bell.event.object_link]                         (J7)
            → [quil.buffer_list.render]                        (K3)
```

The entire J4/J5/J6/J7/K3 chain fires identically to PrintScreen-triggered execution.
**Zero new execution paths.** The palette is purely a UI alternative to the keyboard shortcut.

## STOP FIRST Table

| Item | Why STOP FIRST |
|------|----------------|
| New PDX opcode for palette | ABI edit — sex-pdx crate |
| New display primitive for palette rows | sexdisplay change — renderer policy |
| Text input / fuzzy search | Editor subsystem — text rendering |
| App-provided command manifests | Cross-PD protocol — new PDX ops |
| Real-time command discovery | Active registry — lifecycle change |
| Command history / persistence | Filesystem — storage code |
| Real Bell command events (Bell + palette) | Bell queue real implementation |
| Cross-PD command execution | Each PD owns its actions |
| Palette-specific Collar authority | Collar real grant_refs (STOP FIRST per K2B) |

## K11 Implementation Prompt Summary

```
K11_COMMAND_PALETTE_STUB_IMPLEMENTATION

Patch:
  - servers/silk-shell/src/main.rs
  - docs/handoff/K11_COMMAND_PALETTE_STUB_V1.md

Add to servers/silk-shell/src/main.rs:
  1. const SURFACE_ID_COMMAND_PALETTE: u64 = 0x98;
  2. const MAX_FRAMES from 7→8, ATLAS_MAX_FRAMES_PER_SCENE from 7→8
  3. AppSurfaceSpec entry for command palette (frame_id=7)
  4. Command enum with 5 variants (OpenSelectedInQuil, FocusLinen, FocusQuil,
     SceneNext, OpenAtlas)
  5. static COMMAND_LIST: [CommandDef; 5] with names
  6. palette_selected_index state + navigation helpers
  7. ensure_command_palette_frame(), toggle_command_palette()
  8. SurfaceAction::ToggleCommandPalette + scancode 0x29
  9. palette_render_list() via 0xEF header + proof-marker rows
  10. Selection cycling via J/K (same gate as Linen selection)
  11. Execute dispatches to existing SurfaceAction match arms

Proof markers:
  [palette.open], [palette.close], [palette.row], [palette.select],
  [palette.cmd.execute], [palette.cmd.reject], [palette.done]

Build + commit with message: feat(shell): add command palette stub

After K11: K12 rapid audit K8-K11
```

## Architecture Diagram

```
┌──────────────────────────────────────────┐
│              silk-shell (PKEY 3)          │
│                                          │
│  Keyboard (0x29)                         │
│    └── toggle_command_palette()          │
│          └── palette_open = true         │
│                └── palette_render_list() │
│                      └── 0xEF header     │
│                      └── proof markers   │
│                                          │
│  J/K (0x24/0x25) while palette open      │
│    └── palette_select_next/prev()        │
│          └── palette_render_list()       │
│                                          │
│  Enter (0x1C) while palette open         │
│    └── dispatch to SurfaceAction         │
│          └── existing action handler     │
│                └── J4/J5/J6/J7/K3 chain  │
│                └── focus/lifecycle gates │
│                                          │
│  No new authority. No new PDX.           │
│  No sexdisplay changes. No kernel.       │
└──────────────────────────────────────────┘
```

This is the same architecture as the existing Scene Settings panel or Atlas overlay —
a shell-owned surface that provides an alternative UI for existing shell actions.
