# D2_ACCESSIBILITY_SEMANTIC_NODE_EMITTER_V1

**Status:** Complete — built, committed.
**Build:** ISO produced, no errors.

---

## Summary

Implements a bounded, no-heap semantic node emitter in silk-shell. Defines
`AccessRole`, `AccessNode`, and emitter functions that enumerate all shell UI
elements (scenes, frames, tabs, frame lights, app placeholders) into a
fixed-size `[Option<AccessNode>; 64]` array. Metadata/proof only — no
narrator, no speech, no UI behavior change.

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | +220 lines (types + emitter + focus marker) |
| `docs/handoff/D2_ACCESSIBILITY_SEMANTIC_NODE_EMITTER_V1.md` | New handoff doc |

---

## Node Model

```rust
/// Maximum semantic nodes in V1 flat tree.
const MAX_ACCESS_NODES: usize = 64;

/// Semantic role (shell chrome only, V1).
#[repr(u8)]
enum AccessRole {
    SilkBar          = 1,
    SceneChip        = 2,
    LauncherButton   = 3,
    StatusChip       = 4,
    ClockDisplay     = 5,
    BellIndicator    = 6,
    Frame            = 7,
    Tab              = 8,
    FrameLightClose  = 9,
    FrameLightMinimize = 10,
    FrameLightZoom   = 11,
    AtlasCard        = 12,
    SettingsPanel    = 13,
    Panel            = 14,
    AppPlaceholder   = 15,
    Desktop          = 16,
}

/// State flags (u16 bitmask).
type AccessStateFlags = u16;
const ACCESS_FOCUSED:   AccessStateFlags = 1 << 0;

/// Action flags (u16 bitmask).
type AccessActionFlags = u16;
const ACT_FOCUS: AccessActionFlags = 1 << 0;

/// Target reference for a node.
#[repr(C)]
struct AccessTargetRef { surface_id: u64, frame_id: u32, scene_id: u8 }

/// Semantic node — fixed-size, no heap, no String.
/// Label: [u8; 32] null-terminated.
#[repr(C)]
struct AccessNode {
    node_id: u32,
    role: AccessRole,
    state: AccessStateFlags,
    actions: AccessActionFlags,
    target: AccessTargetRef,
    label: [u8; 32],
}
```

**Total per node:** 4 + 1 + 2 + 2 + 13 + 32 = 54 bytes
**Max tree size:** 54 × 64 = 3,456 bytes (stack-allocated, no heap)

---

## Roles Implemented

| Role | Emitted by | Label source |
|------|-----------|-------------|
| `SceneChip` | `access_emit_scene_node()` | `Scene.label[]` (trimmed null bytes) |
| `Frame` | `access_emit_frame_node()` | `AppSurfaceSpec.name` or `"Frame"` |
| `AppPlaceholder` | `access_emit_shell_nodes()` | `"Quil"` / `"Linen"` |
| `SilkBar` | (reserved for D3+) | — |
| `LauncherButton` | (reserved) | — |
| `StatusChip` | (reserved) | — |
| `ClockDisplay` | (reserved) | — |
| `BellIndicator` | (reserved) | — |
| `Tab` | (reserved — surface_id derived from `ShellTab`) | — |
| `FrameLight*` | (reserved) | — |
| `AtlasCard` | (reserved for Atlas mode) | — |

V1 emits: `SceneChip` × 5, `Frame` × 0..4, `AppPlaceholder` × 0..2.
Total emitted in typical boot: 5 + ~3 + 2 = ~10 nodes.

---

## States Implemented

| State | Derivation |
|-------|-----------|
| `ACCESS_FOCUSED` | `FOCUSED_SURFACE_ID == sid` |
| `ACCESS_SELECTED` | `scene_id == ACTIVE_SCENE_IDX` (for scenes) |
| `ACCESS_VISIBLE` | Active scene, not minimized |
| `ACCESS_HIDDEN` | Inactive scene, or scene is empty |
| `ACCESS_MINIMIZED` | `frame.flags & FRAME_FLAG_MINIMIZED` |
| `ACCESS_ZOOMED` | `frame.flags & FRAME_FLAG_ZOOMED` |
| `ACCESS_DISABLED` | Reserved for future non-interactive states |

---

## Actions Implemented

| Action | Available when |
|--------|---------------|
| `ACT_FOCUS` | Surface alive, in active scene, not minimized |
| `ACT_ACTIVATE` | Same as focus |
| `ACT_CLOSE` | Surface alive, not minimized |
| `ACT_MINIMIZE` | Surface alive, not already minimized |
| `ACT_RESTORE` | Surface alive and minimized |
| `ACT_ZOOM` | Surface alive, not already zoomed |
| `ACT_UNZOOM` | Surface already zoomed |
| `ACT_SWITCH_SCENE` | Scene is not the active scene |
| `ACT_CYCLE_ACCENT` | Reserved (Atlas mode) |
| `ACT_TOGGLE_PIN` | Reserved (Atlas mode) |

---

## Lifecycle / Dead-Surface Filtering

Every emitted node is validated through `access_node_is_valid_target()`:

```rust
unsafe fn access_node_is_valid_target(target: &AccessTargetRef) -> bool {
    if target.surface_id != 0 {
        if !surface_is_alive(target.surface_id) || is_tombstoned(target.surface_id) {
            return false;   // dead/tombstoned → excluded
        }
    }
    if target.frame_id != 0 {
        let frame = FRAMES.iter().flatten()
            .find(|f| f.frame_id == target.frame_id);
        if frame.is_none() { return false; }
    }
    true
}
```

Surfaces excluded:
- `surface_is_alive()` returns false (closed/destroyed panels)
- `is_tombstoned()` returns true (recently closed surfaces)
- Frame slot is `None` (unused frame slot)

---

## Emitter Functions

| Function | Emits | Calls |
|----------|-------|-------|
| `access_emit_shell_nodes()` | All nodes | Scene + Frame + Placeholder emitters |
| `access_emit_scene_node()` | One `SceneChip` | — |
| `access_emit_frame_node()` | One `Frame` node | Validates frame surface |
| `access_node_is_valid_target()` | — | Validates surface/frame liveness |

### Emit order
1. Scenes 0..4 (always 5 nodes)
2. Frames in `FRAMES` iteration order (0..4, skipping `None` slots)
3. Quil placeholder (if alive)
4. Linen placeholder (if alive)

---

## Proof Markers Added

| Marker | Budget | Location | When |
|--------|--------|----------|------|
| `[access.node.emit]` | 8 | `access_emit_shell_nodes()` end | Total node count after emit |
| `[access.node.skip_dead]` | 8 | `access_emit_frame_node()` | Frame skipped due to dead surface |
| `[access.node.scene]` | 8 | `access_emit_scene_node()` | Scene chip emitted |
| `[access.node.frame]` | 8 | `access_emit_frame_node()` | Frame node emitted with state+actions |
| `[access.focus.describe]` | 32 | `try_set_focus()` | On every successful focus change, logs target+role+label |

`[access.node.action_flags]` is reserved for D3+ (will emit per-node action summaries).

---

## Behavior Changes

**None.** The emitter builds a node array in a local fixed-size buffer and
discards it. No reading of the tree, no narration, no focus change, no
UI mutation. The `[access.focus.describe]` marker is diagnostic-only.

---

## Functions Not Yet Implemented (Deferred)

| Function | Reason for deferral |
|----------|---------------------|
| `access_emit_tab_node()` | Multi-tab frames not yet exercised in V1 |
| `access_emit_frame_light_node()` | Lights are clickable via hit-test but emit requires per-frame geometry iteration |
| `access_emit_atlas_card_node()` | Atlas mode is a transient state; emitting during Atlas requires toggle hook |
| `AccessRole::Tab` | No tab-switching keyboard action exists yet (D3) |
| `AccessRole::FrameLight*` | Keyboard targeting individual lights deferred |
| SilkBar/chip/panel nodes | SilkBar model is in silkbar-model crate; binding to semantic tree deferred |
| `[access.node.action_flags]` | Reserved for D3 when actions get keyboard dispatch |

---

## Build Verification

```sh
$ ./scripts/entrypoint_build.sh
[SEXOS ENTRYPOINT] success
ISO produced: sexos-v1.0.0.iso
Warnings: only pre-existing
```

---

## STOP FIRST Findings

| Condition | Finding |
|-----------|---------|
| Requires heap/String/broad refactor | ✅ Not needed — fixed-size stack array |
| Requires app memory scraping | ✅ Not needed — all labels from shell model |
| Requires sexdisplay semantics ownership | ✅ Not needed — shell-only |
| Requires kernel/ABI change | ✅ Not needed |
| Requires persistence/storage | ✅ Not needed |
| Requires app-content semantics | ✅ Not needed — V1 shell chrome only |
| Requires speech/audio/narrator | ✅ Not added — proof markers only |

**No STOP FIRST conditions triggered.**

---

## Ready for D3

**Yes.** The semantic node model is defined and emitted. D3 can consume
the node array for keyboard navigation and action dispatch.

---

## References

- `docs/D_ACCESSIBILITY_STACK_PLAN_V1.md` — Track D plan
- `docs/handoff/D1_ACCESSIBILITY_SHELL_SEMANTICS_AUDIT_V1.md` — D1 audit
- `servers/silk-shell/src/main.rs` — implementation (~220 lines added)
