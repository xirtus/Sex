# SCENE_LIFECYCLE_HARDENING_V1

**Status:** Active  
**Purpose:** Harden Silk Scene lifecycle so active scene, frame membership, visibility sync, focus cleanup, snapshots, and tiling are deterministic and invariant-safe.  
**Scope:** `servers/silk-shell/src/main.rs` only. No kernel/ABI/sexdisplay changes.  
**Prerequisites:** APP_SURFACE_HELPER_DEDUP_PLAN_V1 (ba98620)

---

## 1. Scene Lifecycle Invariants

```
┌─────────────────────────────────────────────────┐
│            Scene Lifecycle Invariants            │
├─────────────────────────────────────────────────┤
│ I1: Exactly one ACTIVE_SCENE_IDX (0..4)         │
│ I2: Every live frame has a scene_id             │
│ I3: Wrong-scene surfaces cannot receive focus   │
│ I4: Wrong-scene surfaces cannot be hovered      │
│ I5: Wrong-scene surfaces cannot be dragged      │  ← NEW
│ I6: Minimized frames are hidden from tiling     │
│ I7: Scene switch clears stale focus/hover/drag  │
│ I8: Visibility synced before tiling             │
│ I9: snap_capture_layout() after every mutation  │
│ I10: No tombstoned surface becomes active       │
└─────────────────────────────────────────────────┘
```

### Invariant I5 (new)

After a scene switch, if a surface from the old scene was being dragged, the drag state must be cleared. Without this, mouse release would act on a now-invisible surface.

---

## 2. Imperfections Found

| # | Issue | Location | Severity |
|---|-------|----------|----------|
| 1 | **Missing `clear_drag_if_wrong_scene()`** — drag state survives scene switch if dragged surface belongs to old scene. Focus and hover both had wrong-scene cleanup; drag did not. | `switch_scene()`, SilkBar handler, `snap_restore_layout()` | Medium |
| 2 | **Missing `[shell.scene.visibility]` marker** — `sync_scene_visibility()` had no log output, making it impossible to trace hide/show decisions from serial log. | `sync_scene_visibility()` | Low |

---

## 3. Patch Summary

### `clear_drag_if_wrong_scene()` — new function

```rust
unsafe fn clear_drag_if_wrong_scene() {
    if let InteractionState::Dragging { surface_id, .. } = INTERACTION {
        if !surface_in_active_scene(surface_id) {
            // budgeted [shell.scene.drag.clear.wrong-scene] marker
            try_transition(InteractionState::Idle);
        }
    }
}
```

### Call sites (3 additions)

| Call site | Why |
|-----------|-----|
| `switch_scene()` (shortcut path) | After `clear_drag_if_dead()`, before `clear_hover_if_wrong_scene()` |
| SilkBar `SwitchWorkspace` handler | Same order — after dead check, before hover cleanup |
| `snap_restore_layout()` | Same order — after dead check, before hover cleanup |

### `sync_scene_visibility()` — added budgeted marker

```
[shell.scene.visibility] sync  (budget 8)
```

---

## 4. Negative-Case Checklist

| Scenario | Behavior | Status |
|----------|----------|--------|
| Switch scene while focused frame in old scene | Focus cleared to alive surface in new scene | ✅ I3 |
| Switch scene while hover active on old-scene frame | Hover cleared | ✅ I4 |
| **Switch scene while dragging old-scene surface** | **Drag cancelled** | ✅ **NEW I5** |
| Minimized frame in old scene | Hidden via 0xEE, excluded from tiling | ✅ I6 |
| Quil/Linen opened then scene switched | Surface hidden, focus falls back | ✅ I3, I8 |
| Restore minimized frame in different scene | Surface shown via 0xEC, focus set | ✅ I7 |
| Tombstoned surface during snapshot restore | Skipped (frame-level) | ✅ I10 |
| Drag dead surface (surface destroyed during drag) | Cancelled by `clear_drag_if_dead()` | ✅ |

---

## 5. Files Changed

- `servers/silk-shell/src/main.rs` — +13 lines for `clear_drag_if_wrong_scene()`, +3 call sites, +1 visibility marker

## 6. Build Result

```
[SEXOS ENTRYPOINT] success
All pipeline stages passed. No new warnings.
```

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Add `clear_drag_if_wrong_scene()`, visibility marker, invariant I5 | SCENE_LIFECYCLE_HARDENING_V1 |
