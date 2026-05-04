# QUIL_SURFACE_STUB_V1

**Status:** Active  
**Purpose:** Allocate Quil surface identity and shell lifecycle matching the Linen control pattern.  
**Scope:** `servers/silk-shell/src/main.rs` only. No kernel/ABI/sexdisplay changes.  
**Reference:** `rapid/PHASE_05_QUIL_LANGUAGE_WORKSTATION.md` (full plan for Quil language workstation)

---

## What Was Done

Quil (`surface_id=201`) is now a first-class shell-managed app surface, following the identical pattern established by Linen (`surface_id=200`). No Quil server binary exists yet — this is pure shell surface/control integration.

### Surface Identity

| Property | Value |
|----------|-------|
| `SURFACE_ID_QUIL` | `201` |
| `QUIL_FRAME_ID` | `3` (frame 1 = APP/STATUS, frame 2 = Linen) |
| Boot geometry | `(100, 100, 640, 480)` |
| Frame flags | `FRAME_FLAG_TOP_BAR` (matching default) |
| Surface lifecycle | Always alive (not destroyable, like Linen) |

### Wiring Changes (12 locations)

| Location | Change |
|----------|--------|
| `SURFACE_ID_QUIL` constant | Added next to `SURFACE_ID_LINEN` |
| Surface ID registry comment | Added Linen/Quil entries |
| `SURFACE_201_X/Y/W/H` statics | Geometry tracking |
| `tile_visible_frames()` | QUIL match arm for tiling position |
| `emit_snapshot()` | QUIL `OP_SURFACE_UPDATE` for position sync |
| `get_surface_bounds()` | QUIL returns geometry |
| `point_in_surface()` | QUIL bounds check |
| `surface_is_alive()` | Returns `true` always (like Linen) |
| `is_focusable_surface()` | QUIL is focusable |
| `is_closeable_surface()` | QUIL cannot be closed (OS-managed) |
| `update_local_geometry()` | QUIL geometry sync |
| `z_order` arrays (×2) | QUIL in focus fallback order |

### New Helpers (5, matching Linen pattern)

| Helper | Description |
|--------|-------------|
| `ensure_quil_frame()` | Creates `ShellFrame` with `frame_id=3` lazily. Returns `Some(3)` or `None`. |
| `open_quil_in_active_scene()` | Opens Quil in current scene (un-minimize, 0xEC, tile, focus). |
| `focus_or_open_quil()` | Focus if visible, else open. |
| `toggle_quil()` | Toggle minimize/restore. |
| `quil_frame_id()` | Returns `Some(3)` or `None`. |

All helpers are lazy — Quil only enters `FRAMES` on first open. Zero boot visual change.

---

## Alignment with PHASE_05 Plan

The existing `rapid/PHASE_05_QUIL_LANGUAGE_WORKSTATION.md` says:

> `SURFACE_ID_QUIL` not allocated → **Now allocated (201)**
> "Create a Quil surface that receives keyboard input..." → **Surface lifecycle ready**
> "Quil PDX server boots and creates a surface on launch" → **Stub ready, real server deferred**

Next step per PHASE_05: create `servers/quil/src/main.rs` as a PDX server that owns surface 201, receives keyboard input, and displays typed characters as colored rectangles.

---

## Build Result

```
Finished release profile [optimized] in 0.78s
Warnings: 217 (all pre-existing)
Errors: 0
```

## Files Changed

- `servers/silk-shell/src/main.rs` (+190 lines for Quil wiring + helpers)

## Not In Scope (deferred to PHASE_05)

- `servers/quil/` server binary
- Keyboard input routing to Quil surface
- Text/Code/Sex mode implementations
- Sex Inspector panels
- Project tree via Linen

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Quil surface stub matching Linen pattern | QUIL_SURFACE_STUB_V1 |
