# E1: Quil Placeholder Surface Through Scene/Frame/Tab

**Status:** Approved
**Commit:** *(pending)*
**Build:** Passed (ISO produced)
**Behavior:** Unchanged (additive proof markers only)

## Purpose

Add proof markers tracing the Quil placeholder surface through the proven
Scene/Frame/Tab lifecycle path — mirroring the exact D1 Linen pattern.
No real editor, no language workstation features, no storage/filesystem work.

## Background

Quil already had full infrastructure (mirroring Linen):
- `SURFACE_ID_QUIL = 201` and `QUIL_FRAME_ID = 3`
- `ensure_quil_frame()` creates a ShellFrame + tab in FRAMES
- `open_quil_in_active_scene()` opens/focuses/tiles via lifecycle FSM
- `toggle_quil()` (F9 key) toggles minimize/open
- Lifecycle registered as `Visible` in `lifecycle_init_all()`
- Focusable via `app_surface_spec()` (focusable: true in APP_SURFACES)
- Placeholder fill rect via `QUIL_PLACEHOLDER_COLOR`

E1 adds explicit proof markers matching D1 exactly.

## Changes to `servers/silk-shell/src/main.rs`

### 1. `ensure_quil_frame()` — Frame/tab attach markers
- Added `[quil.placeholder.attach.frame]` when new ShellFrame slot is created
- Added `[quil.placeholder.attach.tab]` when tab[0] is attached with QUIL surface_id

### 2. `open_quil_in_active_scene()` — Duplicate guard + open/focus markers
- **Duplicate guard** (new, at function start): checks if Quil frame already
  exists in active scene and is not minimized. If so, emits
  `[quil.placeholder.reject.duplicate]` and focuses existing Quil instead.
- `[quil.placeholder.focus]` emitted after successful `try_set_focus()`
- `[quil.placeholder.open]` emitted after successful open

## Proof Markers Added

| Marker | Location | Trigger |
|--------|----------|---------|
| `[quil.placeholder.attach.frame]` | ensure_quil_frame() | New ShellFrame slot for Quil |
| `[quil.placeholder.attach.tab]` | ensure_quil_frame() | Tab[0] attached with QUIL surface_id |
| `[quil.placeholder.reject.duplicate]` | open_quil_in_active_scene() | Quil already visible in active scene |
| `[quil.placeholder.open]` | open_quil_in_active_scene() | Quil successfully opened |
| `[quil.placeholder.focus]` | open_quil_in_active_scene() | Quil surface focused via try_set_focus |

## Invariants

1. Exact mirror of D1 Linen pattern — same guard order, same marker placement
2. Only one Quil frame/tab exists (fixed QUIL_FRAME_ID=3) — duplicate guard catches
   the "already visible" case and refocuses rather than opening
3. Quil lifecycle is `Visible` at boot — all existing FSM transitions
   (minimize/restore/close/tombstone) apply through A5/A6/B3
4. Focus goes through `try_set_focus()` — B2 scene guard + lifecycle guard
5. Tiling goes through `tile_active_scene_frames()` — B3 lifecycle-aware
6. No sexdisplay changes, no ABI edits, no kernel changes

## Deferred

- E2: Quil runtime proof (open/minimize/restore/close/focus/Atlas snapshot)
- Real Quil editor/workstation (language model, parser, compiler)
- Multiple Quil tabs

## Dependencies

- **Requires:** D1 (Linen pattern — exact mirror), A3/A4/A5/A6/B1/B2/B3
- **Blocks:** E2 (runtime proof)
