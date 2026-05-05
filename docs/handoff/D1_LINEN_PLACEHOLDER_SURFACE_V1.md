# D1: Linen Placeholder Surface Through Scene/Frame/Tab

**Status:** Approved
**Commit:** *(pending)*
**Build:** Passed (ISO produced)
**Behavior:** Unchanged (additive proof markers only)

## Purpose

Add proof markers tracing the Linen placeholder surface through the existing
Scene/Frame/Tab lifecycle path. No real file browser, no storage/filesystem
work, no renderer/ABI/kernel changes.

## Background

Linen already has full infrastructure:
- `SURFACE_ID_LINEN = 200` and `LINEN_FRAME_ID = 2`
- `ensure_linen_frame()` creates a ShellFrame + tab in FRAMES
- `open_linen_in_active_scene()` opens/focuses/tiles via lifecycle FSM
- `toggle_linen()` (F8 key) toggles minimize/open
- Lifecycle registered as `Visible` in `lifecycle_init_all()`
- Focusable via `app_surface_spec()` (focusable: true in APP_SURFACES)

D1 adds explicit proof markers for the Scene/Frame/Tab lifecycle path.

## Changes to `servers/silk-shell/src/main.rs`

### 1. `ensure_linen_frame()` — Frame/tab attach markers
- Added `[linen.placeholder.attach.frame]` when new ShellFrame slot is created
- Added `[linen.placeholder.attach.tab]` when tab[0] is attached with LINEN surface_id
- These trace the B1 frame.core.attach / tab.core.attach path for Linen

### 2. `open_linen_in_active_scene()` — Duplicate guard + open/focus markers
- **Duplicate guard** (new, at function start): checks if Linen frame already
  exists in active scene and is not minimized. If so, emits
  `[linen.placeholder.reject.duplicate]` and focuses existing Linen instead.
- `[linen.placeholder.focus]` emitted after successful `try_set_focus()`
- `[linen.placeholder.open]` emitted after successful open (before snap_capture_layout)

## Proof Markers Added

| Marker | Location | Trigger |
|--------|----------|---------|
| `[linen.placeholder.attach.frame]` | ensure_linen_frame() | New ShellFrame slot for Linen |
| `[linen.placeholder.attach.tab]` | ensure_linen_frame() | Tab[0] attached with LINEN surface_id |
| `[linen.placeholder.reject.duplicate]` | open_linen_in_active_scene() | Linen already visible in active scene |
| `[linen.placeholder.open]` | open_linen_in_active_scene() | Linen successfully opened |
| `[linen.placeholder.focus]` | open_linen_in_active_scene() | Linen surface focused via try_set_focus |

## Invariants

1. Only one Linen frame/tab exists (fixed LINEN_FRAME_ID=2) — duplicate guard catches
   the "already visible" case and refocuses rather than opening
2. Linen lifecycle is `Visible` at boot — all existing FSM transitions
   (minimize/restore/close/tombstone) apply through A5/A6/B3
3. Focus goes through `try_set_focus()` — B2 scene guard + lifecycle guard
   ensure active-scene only, visible/mapped states only
4. Tiling goes through `tile_active_scene_frames()` — B3 lifecycle-aware
5. No sexdisplay changes, no ABI edits, no kernel changes

## Deferred

- D2: Linen placeholder runtime proof (open/minimize/restore/close/focus/Atlas)
- D3: Linen object model docs only
- Real Linen server protocol (storage/filesystem)
- Multiple Linen tabs

## Dependencies

- **Requires:** A3 (lifecycle model), A4 (focus guards), A5 (frame lights),
  A6 (tombstone events), B1 (Scene/Frame/Tab model), B2 (scene focus guards),
  B3 (lifecycle-aware tiling)
- **Blocks:** D2 (runtime proof)
