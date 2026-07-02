# ATLAS_FRAME_PREVIEW_REFRESH_V1

**Status:** Complete — built, committed.
**Build:** ISO produced, no errors.

---

## Summary

Adds lifecycle-aware filtering to Atlas frame previews. Frames whose active tab surface is dead (tombstoned/destroyed/closing) are now excluded from `scene_update_flags()` and `atlas_capture_snapshot()`. This prevents Atlas from showing stale frame indicator blocks for surfaces that no longer exist.

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | +22 lines (lifecycle filtering in 2 functions + 3 proof markers) |
| `docs/handoff/ATLAS_FRAME_PREVIEW_REFRESH_V1.md` | New handoff doc |

---

## Changes Detail

### 1. `scene_update_flags()` — lifecycle filter

Before: counted all frames matching scene_id, regardless of surface state.

After: skips frames whose active tab surface is dead or tombstoned:

```rust
// A8+: Skip frames whose active tab surface is dead or tombstoned.
if let Some(sid) = active_surface_for_frame(frame.frame_id) {
    if !surface_is_alive(sid) || is_tombstoned(sid) {
        continue;
    }
}
```

This ensures `SCENE_FLAG_EMPTY`, `SCENE_FLAG_HAS_MINIMIZED`, and `SCENE_FLAG_HAS_ZOOMED` only reflect frames with live surfaces.

### 2. `atlas_capture_snapshot()` — lifecycle filter

Before: added all frames with matching scene_id to `frame_ids[]`.

After: skips frames whose active tab is dead or tombstoned, with proof marker:

```rust
// A8+: Skip frames whose active tab is dead or tombstoned.
if let Some(sid) = active_surface_for_frame(frame.frame_id) {
    if !surface_is_alive(sid) || is_tombstoned(sid) {
        // [atlas.preview.skip_dead] marker
        continue;
    }
}
```

This ensures `frame_count` and `frame_ids[]` only contain frames with live surfaces.

### 3. `atlas_render_stub()` — refresh marker

Added `[atlas.preview.refresh]` marker alongside existing `[shell.atlas.render]`.

### 4. `atlas_capture_snapshot()` — scene preview marker

Added per-scene `[atlas.preview.scene]` marker showing frame count and flags after lifecycle filtering.

---

## Proof Markers

| Marker | Location | Budget | When |
|--------|----------|--------|------|
| `[atlas.preview.skip_dead]` | `atlas_capture_snapshot()` | 8 | Frame skipped due to dead/tombstoned surface |
| `[atlas.preview.refresh]` | `atlas_render_stub()` | 4 | Atlas overlay rendered with fresh previews |
| `[atlas.preview.scene]` | `atlas_capture_snapshot()` | 4 per scene | Per-scene frame count and flags |

---

## Stale/Dead Filtering Summary

| Surface State | Included in Atlas preview? | Why |
|---------------|---------------------------|-----|
| Visible, active scene | ✅ Yes | Live, focusable content |
| Hidden, inactive scene | ✅ Yes | Live, valid scene member |
| Minimized | ✅ Yes | Existing Atlas semantics preserve minimized representation |
| Zoomed | ✅ Yes | Existing Atlas semantics preserve zoomed indicator |
| Closing | ❌ No | Surface mid-close — excluded by `surface_is_alive()` returning false |
| Tombstoned | ❌ No | Surface dead — excluded by `is_tombstoned()` |
| Destroyed | ❌ No | Surface dead — excluded by `surface_is_alive()` returning false |
| Allocated (panel) | ❌ Not in FRAMES | Panel surfaces don't have frames, not iterated |

---

## Behavior Changes

- **None for live surfaces.** Frames with live, non-tombstoned surfaces appear exactly as before.
- **Dead/tombstoned surfaces no longer shown** as frame indicator blocks in Atlas previews.
- **Scene-level flags** (`SCENE_FLAG_EMPTY`, `SCENE_FLAG_HAS_MINIMIZED`, `SCENE_FLAG_HAS_ZOOMED`) now reflect only frames with live surfaces.

---

## Build Verification

```sh
./scripts/entrypoint_build.sh
# Result: ISO produced, no errors
```

---

## STOP FIRST Findings

None. This patch:
- Uses existing lifecycle helpers (`surface_is_alive()`, `is_tombstoned()`, `active_surface_for_frame()`)
- Does not change display protocol
- Does not change sexdisplay
- Does not add new UI
- Does not change navigation or selection behavior

---

## Ready for Atlas Scene Settings?

**Yes.** Preview refresh is visually/model correct. Atlas now shows accurate frame counts excluding dead surfaces.

---

## Document References

- `docs/handoff/A8_LIFECYCLE_PROOF_SCENARIOS_V1.md` — lifecycle proof scenarios
- `docs/handoff/HIDDEN_STATE_TRACKING_CLEANUP_V1.md` — Hidden state tracking
- `servers/silk-shell/src/main.rs` — implementation

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Add lifecycle filtering to Atlas frame previews. Skip dead/tombstoned frames in scene flags and snapshot. | ATLAS_FRAME_PREVIEW_REFRESH_V1 |
