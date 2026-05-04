# B4: Tab Chrome + Frame Lights

**Status:** Approved
**Commit:** `0db8983`
**Build:** Passed (ISO produced)
**Behavior:** Unchanged for normal cases; chrome visibility now correctly restricted for inactive/dead/minimized/tombstoned frames.

## Purpose

Wire tab strip visibility, hover-reveal, and Frame Light behavior onto the B1/B2/B3 Scene/Frame/Tab model.
Shell policy only. Renderer remains pixel-only.

## Principle

**silk-shell derives chrome state; sexdisplay only draws what it is told.**
Chrome visibility is a shell-side policy decision, packed into `OP_SURFACE_TAB_INFO`
as a chrome_visible bit. Sexdisplay receives the bit and renders accordingly — it
never decides when chrome should be visible.

## Changes to `servers/silk-shell/src/main.rs`

### 1. `frame_chrome_visible()` — new helper (line 3398)

Derives tab chrome visibility per-frame:

| Condition | Result |
|-----------|--------|
| Frame in inactive scene | hidden |
| Frame minimized | hidden |
| Active tab surface dead | hidden |
| Active tab surface tombstoned | hidden |
| Multi-tab (tab_count > 1) | always visible |
| Single-tab + hovered by pointer | visible |
| Single-tab + not hovered | hidden |

### 2. `send_frame_tab_info()` — chrome_visible bit (line 3984)

Bit 9 of arg2 now carries `chrome_visible`:
- Bit 0-7: active_tab index
- Bit 8: top_bar flag
- **Bit 9: chrome_visible flag**

### 3. `update_frame_hover_at()` — chrome proof markers (line 4204+)

After hover state changes, derives chrome visibility for the hovered frame:

- Single-tab frame hovered → `[tab.chrome.show]` + `send_frame_tab_info()`
- Single-tab frame unhovered → `[tab.chrome.hide]` + `send_frame_tab_info()`
- Multi-tab frame hovered → `[tab.chrome.persist.multi]` (no send needed)

### 4. Frame Light reject guards (line 4642+)

All Frame Light actions (close/minimize/zoom) now check frame validity:

| Reject Reason | Guard | Marker |
|--------------|-------|--------|
| Inactive scene | `frame.scene_id != ACTIVE_SCENE_IDX` | `[frame.light.reject.inactive]` |
| Minimized | `FRAME_FLAG_MINIMIZED` | `[frame.light.reject.inactive]` |
| Dead surface | `!surface_is_alive()` | `[frame.light.reject.inactive]` |
| Tombstoned surface | `is_tombstoned()` | `[frame.light.reject.inactive]` |

## Frame Light Actions (via existing A5/A6 FSM)

| Light | Action | FSM Path | Tiling |
|-------|--------|----------|--------|
| Red (CLOSE) | `close_surface_from_frame_light()` | A5/A6: →Closing→Tombstoned→Destroyed | `tile_active_scene_frames()` |
| Yellow (MINIMIZE) | `minimize_frame()` | A3: →Minimized | `tile_active_scene_frames()` |
| Green (ZOOM) | `toggle_zoom_frame()` | A5: →zoomed/→unzoomed | `tile_active_scene_frames()` |

## Proof Markers

| Marker | Location | Trigger |
|--------|----------|---------|
| `[tab.chrome.show]` | update_frame_hover_at() | Single-tab chrome revealed on hover |
| `[tab.chrome.hide]` | update_frame_hover_at() | Single-tab chrome hidden on hover leave |
| `[tab.chrome.persist.multi]` | update_frame_hover_at() | Multi-tab chrome confirmed visible |
| `[frame.light.reject.inactive]` | FrameChrome click dispatch | Action on inactive/dead/minimized/tombstoned |

## Invariants

1. Chrome visibility is always derived by silk-shell, never by sexdisplay
2. Multi-tab frames always show chrome; single-tab only on hover
3. Inactive, minimized, dead, and tombstoned frames never show chrome
4. Frame Lights (close/minimize/zoom) only fire for valid, active-scene frames
5. All Frame Light actions go through the A5/A6 lifecycle FSM paths
6. Tiling is re-applied after close/minimize/restore/zoom

## Deferred

- C1: Atlas snapshot/view (next phase)

## Dependencies

- **Requires:** B1 (Scene/Frame/Tab), B2 (active-scene focus), B3 (deterministic tiling)
- **Blocks:** C1 (Atlas)
