# SCENE_PERSISTENCE_REFINED_PLAN_V1

## Status

Design (2026-05-04). Refined persistence plan for Silk Scene/Frame/Tab layout
state, based on audit of existing `docs/SCENE_PERSISTENCE_PLAN_V1.md` against
live code. Docs-only — no implementation.

---

## Prerequisites (all met)

| Prerequisite | Status |
|---|---|
| Phase 01 contract gates | ✅ PDX contract stable, sexstore K/V operational |
| Phase 02 Scene/tombstone/frame/tab | ✅ `ACTIVE_SCENE_IDX`, `FRAMES`, `TOMBSTONES` all live |
| TILING_ENGINE_V1 | ✅ `tile_visible_frames()` implements layout |
| CHROME_TEMPLATE_V1 | ✅ `ChromeTemplate` + `SILK_CHROME_TEMPLATE_DEFAULT` present |
| SCENE_APPEARANCE_PERSIST_V1 | ✅ Scene settings persisted (key 0x01) |

---

## Corrections to Existing Plan

| Issue | Plan Says | Code Reality | Correction |
|-------|-----------|--------------|------------|
| `FrameSnapshot.frame_id` type | `u8` | `ShellFrame.frame_id: u32` | Safe: MAX_FRAMES=4, u8 suffices for snapshot. Document truncation. |
| Frame struct size | ~156 bytes | ~224 bytes (ShellFrame + ShellTab array) | Update size calc, max ~880 bytes |
| scene_id validation | Not specified | `WORKSPACE_COUNT = 5` (silkbar-model) | Add local const `WORKSPACE_COUNT: usize = 5` |
| Geometry clamp source | "screen bounds" | `DesktopPolicy` values | Use `P.width`, `P.height`, `P.bar_height`, `P.min_width`, `P.min_height` |
| ChromeTemplate persistence | Listed as serializable | Compile-time `const`; only `FRAME_FLAG_TOP_BAR` is per-instance | Clarify: ChromeTemplate is invariant, only flag state persists |
| Naming collision | `SceneSnapshot` / `SCENE_SNAPSHOT` | Existing `SNAPSHOT: [WindowDescriptor; 16]` + `emit_snapshot()` | Use `SceneLayoutSnap` / `SCENE_LAYOUT_SNAP` to disambiguate |
| WORKSPACE_COUNT import | Not addressed | Not imported in silk-shell | Define local `const WORKSPACE_COUNT: usize = 5` |

---

## 1. Exact Recommended Structs

### V1a in-memory snapshot (packed, no padding)

```rust
/// Scene layout snapshot header + frame array.
/// V1a in-memory only. NOT a WindowDescriptor snapshot.
#[derive(Clone, Copy)]
#[repr(C, packed)]
struct SceneLayoutSnap {
    magic: u8,              // 0xAC
    version: u8,            // 0x01
    generation: u8,         // monotonic; detect stale
    active_scene_id: u8,    // 0..WORKSPACE_COUNT-1
    frame_count: u8,        // valid entries in frames[] (0..4)
    _pad: [u8; 3],          // reserved, zero
    frames: [FrameLayoutSnap; 4],
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct FrameLayoutSnap {
    frame_id: u8,           // historical/debug, NOT restored as live ID
    scene_id: u8,           // 0..WORKSPACE_COUNT-1
    flags: u32,             // FRAME_FLAG_MINIMIZED | ZOOMED | TOP_BAR
    normal_x: i32,          // pre-zoom geometry
    normal_y: i32,
    normal_w: u32,
    normal_h: u32,
    active_tab: u8,         // index into tabs[]
    tab_count: u8,          // valid tabs (1..8; 0 = reject frame)
    _pad: [u8; 2],
    tabs: [TabEntrySnap; 8],
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
struct TabEntrySnap {
    app_hint: u64,          // historical only, NOT restored as live SurfaceId
    title_id: u64,          // reserved
    flags: u32,             // reserved
    _pad: [u8; 4],
}
```

### Size: 880 bytes max

| Component | Size |
|---|---|
| Header | 8 bytes |
| Per frame (packed) | 218 bytes |
| 4 frames max | 872 bytes |
| **Total max** | **880 bytes** |

No heap allocation. Static buffer via `core::mem::MaybeUninit`.

---

## 2. What Persists vs Does Not

### Persists

| Field | Source | Validation |
|-------|--------|------------|
| `active_scene_id` | `ACTIVE_SCENE_IDX` | Clamp to `WORKSPACE_COUNT-1` |
| `frame_count` | Count of `Some` entries in `FRAMES` | `<= MAX_FRAMES` |
| `frame_id` | `ShellFrame.frame_id` | Stored as u8; restore allocates NEW ID |
| `scene_id` | `ShellFrame.scene_id` | Validate `< WORKSPACE_COUNT` |
| `flags` | `ShellFrame.flags` | Mask to MINIMIZED\|ZOOMED\|TOP_BAR only |
| `normal_x/y/w/h` | `ShellFrame.normal_*` | Clamp via `DesktopPolicy` + `clamp_surface_size()` |
| `active_tab` | `ShellFrame.active_tab` | Reset to 0 if `>= tab_count` |
| `tab_count` | `ShellFrame.tab_count` | Clamp to MAX_TABS_PER_FRAME (8) |
| `app_hint` | `ShellTab.surface_id` | Historical reference ONLY; NOT live restore target |
| `title_id` | `ShellTab.title_id` | Opaque |

### Does NOT persist

| Item | Reason |
|------|--------|
| Transient hover/drag state | Ephemeral interaction |
| `FOCUSED_SURFACE_ID` / `FOCUS_ID` | Not deterministic; restored via `clear_focus_if_dead()` + `tile_visible_frames()` |
| Tombstones | Circular debug buffer; not scene state |
| Raw pointers / PD references | Invalid across restart |
| Framebuffer contents | Owned by sexdisplay |
| ChromeTemplate itself | Compile-time const; only `FRAME_FLAG_TOP_BAR` per-instance |
| `SURFACE_*_ALIVE` booleans | Ephemeral; rebuilt from frame/tab model |
| `SNAPSHOT[WindowDescriptor; 16]` | sexdisplay's window list, not shell layout |

---

## 3. Restore Policy

### Validation sequence

1. `SCENE_LAYOUT_SNAP_VALID` check — skip if false
2. **Magic** `!= 0xAC` → discard
3. **Version** `> 0x01` → discard (unknown future format)
4. **Generation** `< current` → discard (stale)
5. **Frame count** `= min(count, 4)`
6. **Per-frame**:
   - `tab_count == 0` → reject frame
   - `tab_count = min(tab_count, 8)`
   - `active_tab >= tab_count` → reset to 0
   - `scene_id >= WORKSPACE_COUNT` → clamp to 0
   - `flags &= (MINIMIZED\|ZOOMED\|TOP_BAR)` (mask unknown bits)
   - MINIMIZED + ZOOMED both set → clear MINIMIZED (conflict resolution)
   - Geometry: `clamp_surface_size(x, y, w, h)` with live `DesktopPolicy`
7. **Frame allocation**: Iterate `FRAMES[]`, assign first `None` slot. Use next `frame_id` counter.
8. **Surface creation**: For each tab, call existing surface creation path (0xEB).
9. **Focus**: `clear_focus_if_dead()` + `clear_focus_if_wrong_scene()` after restore.
10. **Rebuild**: `tile_visible_frames()`.
11. **Generation**: Increment after successful restore.

### Failure policy

| Condition | Behavior |
|---|---|
| All entries invalid | Default Scene 0 boot (current boot behavior) |
| Partial restore | Restore valid frames, skip invalid |
| Corrupt single frame | Skip that frame, continue |
| Missing/no snapshot | Default boot |
| Geometry out of bounds | Clamp via DesktopPolicy |
| No focusable surface | FOCUSED_SURFACE_ID = 0 |
| Generation stale | Discard, default boot |
| Empty frame in snapshot | Reject that frame |

---

## 4. Storage Path

### V1a: In-memory only (same boot)

- `static mut SCENE_LAYOUT_SNAP: MaybeUninit<SceneLayoutSnap>`
- `static mut SCENE_LAYOUT_SNAP_VALID: bool`
- `static mut SCENE_SNAP_GENERATION: u8`
- Survives silk-shell PD restart within same boot
- Written on: frame add/remove, scene switch, minimize/restore, zoom/unzoom, tab switch
- Read at: shell `_start()` before default frame init

### V1b: sexstore-backed (future, blocked)

- Must first pass P6 gate: partial write detection, replay safety, key allocation
- Multi-key: header as `0x02`, frame entries as `0x03+N`
- Keep V1a buffer as write-back cache
- Not before corruption/replay spec written

---

## 5. Implementation Phases

| Phase | Name | Files Touched | Gate |
|-------|------|---------------|------|
| P0 | Audit | `docs/` only | Verify ShellFrame/ShellTab layout, WORKSPACE_COUNT, frame_id counter |
| P1 | Spec handoff | `docs/handoff/SCENE_PERSISTENCE_SPEC_V1.md` | Structs frozen, validation rules documented |
| P2 | V1a Capture | `servers/silk-shell/src/main.rs` | `snap_capture_layout()` on state change events |
| P3 | V1a Validation | `servers/silk-shell/src/main.rs` | `snap_validate()` — no behavioral change yet |
| P4 | V1a Restore | `servers/silk-shell/src/main.rs` | `snap_restore()` in `_start()` before default init |
| P5 | V1a Proof | `servers/silk-shell/src/main.rs` | 12 scenarios, boot proof markers |
| P6 | V1b Storage Gate | `docs/` only | Partial write/replay/corruption spec required |
| P7 | V1b sexstore | `servers/sexstore/`, `servers/silk-shell/` | Only after P6 gate approved |

---

## 6. STOP FIRST Triggers

| # | Trigger | Action |
|---|---------|--------|
| 1 | Kernel edit | HALT — any change to `kernel/` |
| 2 | PDX ABI edit | HALT — any change to `crates/sex-pdx/` |
| 3 | sexdisplay render path change | HALT — display protocol untouched |
| 4 | PD resurrection / session restore | HALT — scene persistence ≠ process revive |
| 5 | sexstore writes before P6 gate | HALT — corruption/replay spec required first |
| 6 | Capability handle in snapshot | HALT — invalid across restart |
| 7 | Raw pointer in snapshot | HALT — invalid across restart |
| 8 | POSIX path/process assumptions | HALT — no_std microkernel |
| 9 | Broad shell layout refactor | HALT — incremental only |
| 10 | Heap allocation for serialization | HALT — 880-byte static buffer sufficient |
| 11 | Non-shell PD writing snapshot | HALT — shell owns exclusively |
| 12 | Inferring identity from app_hint | HALT — historical reference only |
| 13 | ChromeTemplate in snapshot | SOFT FLAG — only FRAME_FLAG_TOP_BAR persists |
| 14 | sexstore key allocation undocumented | SOFT FLAG — allocate keys in plan doc |

---

## 7. Proof Markers

```
[shell.scene.persist.capture]      frames=N active_scene=S gen=G
[shell.scene.persist.validate]     ok=1|reason=corrupt|empty|stale|version
[shell.scene.persist.restore]      frames=N active_scene=S gen=G ok=1|skipped
[shell.scene.persist.restore.frame] id=N scene=S flags=M ok=1|skipped
[shell.scene.persist.restore.focus] cleared=1
```

---

## 8. Key Design Decisions

1. **`repr(C, packed)`** — avoids padding ambiguity. x86-64 handles unaligned loads. Simpler than manual serialize/deserialize.

2. **`MaybeUninit` for buffer** — explicit `SCENE_LAYOUT_SNAP_VALID` flag for safety.

3. **`app_hint` is NOT a SurfaceId** — V1a restore creates new ShellTab with `surface_id: 0`. app_hint exists solely for debugging. Live surfaces must be renegotiated through sexdisplay lifecycle.

4. **Generation counter is u8 (wrapping)** — max 255 restores per boot. Safe because check is `>= current`, not equality.

5. **No ChromeTemplate in snapshot** — compile-time const. Only `FRAME_FLAG_TOP_BAR` is per-instance.

6. **Name collision avoidance**: `SceneLayoutSnap` not `SceneSnapshot`, `SCENE_LAYOUT_SNAP` not `SCENE_SNAPSHOT`. Existing `SNAPSHOT` / `emit_snapshot()` is for sexdisplay window descriptors.

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/SCENE_PERSISTENCE_PLAN_V1.md` | Original plan (corrections documented above) |
| `docs/handoff/CHROME_TEMPLATE_V1.md` | ChromeTemplate (compile-time, not persisted) |
| `docs/handoff/SCENE_SETTINGS_PERSIST_V1.md` | Scene blob persistence (key 0x01, precedent for key 0x02+) |
| `docs/handoff/SEXSTORE_KV_RAM_V1.md` | sexstore K/V constraints (8-byte value limit) |
| `servers/silk-shell/src/main.rs` | All implementation changes target this file |

## Next Recommended Phase

**P0: Audit** — Verify `ShellFrame`/`ShellTab` field layout, confirm `frame_id` allocation pattern, import or define `WORKSPACE_COUNT` locally. Then **P1: Spec handoff** (`docs/handoff/SCENE_PERSISTENCE_SPEC_V1.md`) with frozen structs.
