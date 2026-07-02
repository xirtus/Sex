# QUIL_VISUAL_PLACEHOLDER_V1

**Status:** Active  
**Purpose:** Make the Quil surface visually distinguishable as a placeholder (no real server yet).  
**Scope:** `servers/silk-shell/src/main.rs` only. No kernel/ABI/sexdisplay changes.  

---

## Problem

Quil (surface_id=201) has no server implementation yet. When F9 opens the
Quil surface, sexdisplay creates it via the 0xEC upsert path using a default
color based on surface_id parity: `0x00704890` (purple, since 201 is odd).

This default color makes Quil look like any other active surface, which is
misleading — there is no real app serving content. The user needs a visual
cue that Quil is a placeholder.

## Solution

After creating or updating the Quil surface geometry (via 0xEC), the shell
sends a 0xEF (OP_SURFACE_FILL_RECT) opcode to fill the entire surface with
a distinctive dark slate blue-gray color (`0x0018202E`). This overrides the
default parity-based color with a recognizable "empty workspace" appearance.

### Design Choice: Why 0xEF instead of changing 0xEC?

- 0xEC's color assignment is hardcoded in sexdisplay by surface_id parity
- 0xEF is the existing protocol for per-surface fill rectangles
- No sexdisplay changes needed — the shell already communicates via PDX
- The fill rect persists until the surface is destroyed (survives retile)

### Fill coverage

The fill rect is set in two places to cover all show paths:

| Call site | Coverage |
|-----------|----------|
| `tile_visible_frames()` (after 0xEC) | First open, retile, scene switch, resize |
| `open_quil_in_active_scene()` (before snap_capture_layout) | Restore from minimized — where `tile_visible_frames()` is NOT called |

### Color selection

`0x0018202E` = dark slate blue-gray. Rationale:
- Distinct from all default sexdisplay parity colors (0x00303860, 0x00704890)
- Looks intentionally empty, not like a rendering glitch
- Low saturation — visually recedes, putting focus on active surfaces
- Matches the OS background palette's deep navy aesthetic

## Files Changed

- `servers/silk-shell/src/main.rs` — added `QUIL_PLACEHOLDER_COLOR` constant
  + 0xEF calls in `tile_visible_frames()` and `open_quil_in_active_scene()`

## Build Result

```
Finished dev profile [unoptimized + debuginfo] target(s) in 0.41s
Warnings: 202 (all pre-existing)
Errors: 0
```

## Future (when real Quil server exists)

- Remove `QUIL_PLACEHOLDER_COLOR` and the 0xEF calls
- The real Quil server owns surface_id=201 and sets its own content
- The shell hands off: stops sending 0xEF, resumes normal frame management
- Ownership transfer: shell destroys placeholder (0xEE), Quil server creates
  (0xEC) → becomes new owner in sexdisplay

## Not In Scope

- Named window title or icon in frame chrome
- Progress indicator or "Loading..." animation
- Keyboard shortcut to distinguish placeholder from real app
- Multiple Quil instances

## Next Step

→ `QUIL_SERVER_BOUNDARY_PLAN_V1` — boundary plan for creating `servers/quil`.
  Then `QUIL_SERVER_STUB_PD_V1` — actual no_std PD stub.

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Quil visual placeholder via 0xEF fill rect | QUIL_VISUAL_PLACEHOLDER_V1 |
