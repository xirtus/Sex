# APP_SURFACE_HELPER_DEDUP_PLAN_V1

**Status:** Active  
**Purpose:** Plan for deduplicating Linen/Quil shell lifecycle helpers using the app surface registry.  
**Scope:** Docs only. No implementation.  
**Prerequisites:** APP_SURFACE_LOOKUP_CONFORMANCE_V1 (6ef0f7b)

---

## 1. Duplication Map

### Helper pairs

| Helper family | Linen | Quil | Lines | Identical logic? |
|--------------|-------|------|-------|-----------------|
| `ensure_*_frame()` | 1630-1674 | 1812-1856 | ~44 | ✅ Yes — only frame_id, surface_id, boot_geom, budget name differ |
| `open_*_in_active_scene()` | 1679-1728 | 1861-1912 | ~50 | ⚠️ Mostly — Quil has extra 0xEF fill rect (3 lines) |
| `focus_or_open_*()` | 1733-1754 | 1917-1936 | ~22 | ✅ Yes — only frame_id + budget name differ |
| `toggle_*()` | 1759-1779 | 1941-1959 | ~21 | ✅ Yes — only frame_id + budget name differ |
| `*_frame_id()` | 1782-1791 | 1962-1971 | ~10 | ✅ Yes — only frame_id constant differs |
| **Total** | **~140** | **~140** | **~280** | |

### What varies (the only differences)

| Field | Linen | Quil | Source |
|-------|-------|------|--------|
| frame_id | `LINEN_FRAME_ID = 2` | `QUIL_FRAME_ID = 3` | `AppSurfaceSpec.frame_id` |
| surface_id | `SURFACE_ID_LINEN = 200` | `SURFACE_ID_QUIL = 201` | `AppSurfaceSpec.surface_id` |
| boot_x | `LINEN_BOOT_X = 900` | `QUIL_BOOT_X = 100` | `AppSurfaceSpec.boot_x` |
| boot_y | `LINEN_BOOT_Y = 500` | `QUIL_BOOT_Y = 100` | `AppSurfaceSpec.boot_y` |
| boot_w | `LINEN_BOOT_W = 300` | `QUIL_BOOT_W = 640` | `AppSurfaceSpec.boot_w` |
| boot_h | `LINEN_BOOT_H = 150` | `QUIL_BOOT_H = 480` | `AppSurfaceSpec.boot_h` |
| budget prefix | `linen` | `quil` | Name string in spec |
| extra logic | none | 0xEF fill rect | Quil-only placeholder |

### What is NOT duplicated across pairs

- `update_local_geometry()` — surface-specific match arms for all surfaces, not just Linen/Quil
- `tile_visible_frames()` — single function handling all surfaces
- `surface_is_alive()` — hardcoded per-surface (no registry alive field)
- `get_surface_bounds()` / `point_in_surface()` — per-surface match, geometry only

---

## 2. Staged Refactor Plan

### Phase A: Generic `ensure_app_surface_frame(spec)` (when needed)

Create a single function to replace both `ensure_linen_frame()` and `ensure_quil_frame()`.

```rust
/// Ensure a frame exists for the given app surface spec.
/// Creates frame in first empty FRAMES slot using spec properties.
/// Returns frame_id if created/found, None if no slot.
unsafe fn ensure_app_surface_frame(spec: &AppSurfaceSpec) -> Option<u32> {
    // Check if frame already exists
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if frame.frame_id == spec.frame_id {
                return Some(spec.frame_id);
            }
        }
    }
    // Find empty slot
    for slot in FRAMES.iter_mut() {
        if slot.is_none() {
            *slot = Some(ShellFrame {
                frame_id: spec.frame_id,
                active_tab: 0,
                tab_count: 1,
                tabs: /* tab with spec.surface_id */,
                scene_id: ACTIVE_SCENE_IDX,
                flags: FRAME_FLAG_TOP_BAR,
                normal_x: spec.boot_x,
                normal_y: spec.boot_y,
                normal_w: spec.boot_w,
                normal_h: spec.boot_h,
            });
            return Some(spec.frame_id);
        }
    }
    None
}
```

**Challenges:**
- No budget marker per surface (would need `spec.name` for dynamic logging)
- No budget-per-surface — could use a single shared budget or name-based log
- Tabs array initialization is verbose (fixed `MAX_TABS_PER_FRAME` array with single tab)

**Estimated diff:** −70 lines (replace ~88 lines with ~18-line generic + thin wrappers)

**Risks:** Low. Pure frame struct construction from spec fields.

**STOP FIRST:** If `AppSurfaceSpec` needs heap allocation, dynamic dispatch, or trait objects.

### Phase B: Generic `open_surface_in_active_scene(spec)` (when needed)

Replace both `open_linen_in_active_scene()` and `open_quil_in_active_scene()`.

```rust
unsafe fn open_surface_in_active_scene(spec: &AppSurfaceSpec) -> bool {
    let fid = match ensure_app_surface_frame(spec) {
        Some(f) => f,
        None => return false,
    };
    // Update scene_id
    // Handle minimized/zoomed/visible
    // 0xEC with spec.boot_*
    // tile_visible_frames()
    // try_set_focus
    // snap_capture_layout()
    // budget log with spec.name
}
```

**Challenges:**
- Quil has extra 0xEF fill rect call (3 lines) — would need a callback or flag in spec
- Boot geometry 0xEC is parametric (from spec.boot_*) — fine
- Budget logging needs spec.name — fine

**Estimated diff:** −70 lines

**Risks:** Low-medium. The 0xEF divergence would need either:
- `AppSurfaceSpec.placeholder_color: Option<u32>` field
- Or keep thin wrapper `open_quil_in_active_scene()` that calls generic + 0xEF

**Recommendation:** Keep thin Quil wrapper with 0xEF. Do not add placeholder_color to the spec.

### Phase C: Generic `focus_or_open_surface(spec)` and `toggle_surface(spec)` (when needed)

```rust
unsafe fn focus_or_open_surface(spec: &AppSurfaceSpec) -> bool { ... }
unsafe fn toggle_surface(spec: &AppSurfaceSpec) -> bool { ... }
```

**Challenges:** Budget markers use surface-specific static variables. Could use a single budget with spec.name for log.

**Estimated diff:** −40 lines

**Risks:** Low. Frame lookup by frame_id is identical.

### Phase D: Generic `surface_frame_id(frame_id)` (when needed)

```rust
unsafe fn surface_frame_id(frame_id: u32) -> Option<u32> {
    FRAMES.iter().find_map(|f| {
        f.as_ref().and_then(|frame| {
            if frame.frame_id == frame_id { Some(frame.frame_id) } else { None }
        })
    })
}
```

This one is so trivial it's barely worth abstracting. The `*_frame_id()` wrappers are 10 lines each and clearly document which surface they query.

---

## 3. STOP FIRST Table

| # | Condition | Phase | Rationale |
|---|-----------|-------|-----------|
| 1 | Heap/dynamic registry | Any | AppSurfaceSpec is `const` — must stay compile-time |
| 2 | Trait objects or dyn dispatch | Any | no_std + no trait objects in shell |
| 3 | New frame_id/surface_id allocation | Any | Must come from existing constants, not runtime |
| 4 | Behavioral change in named wrappers | Any | `toggle_linen()` and `toggle_quil()` must remain identical |
| 5 | Removing named wrappers entirely | D | Named wrappers document intent; removal reduces readability |
| 6 | Adding placeholder_color to AppSurfaceSpec | B | Spec is for policy, not visual behavior |

---

## 4. Cost/Benefit Analysis

| Phase | Lines removed | Risk | Readability change |
|-------|-------------|------|-------------------|
| A: ensure frame | −70 | Low | Slight improvement |
| B: open active | −70 | Low-Medium | Slight improvement |
| C: focus/toggle | −40 | Low | Mixed — wrappers are clearer |
| D: frame_id | −20 | Low | Negative — less readable |
| **Total** | **−200** | | |

### Counterargument: why NOT to dedup

1. **Only 2 surfaces exhibit the pattern.** 3+ would justify generics.
2. **Named wrappers are self-documenting.** `toggle_linen()` is clearer than `toggle_surface(app_surface_spec(LINEN_FRAME_ID).unwrap())`.
3. **Budget markers are per-surface.** Abstraction loses named budgets without string-based logging (which is fine but adds parameter).
4. **Quil has divergent behavior** (0xEF fill rect). Requires callback, flag, or thin wrapper.
5. **Surface count is stable.** No pipeline adds new app surfaces weekly. The registry is a safety net, not a driver for refactoring.

### What exists ALREADY avoids duplication in the right places

The shell already avoids duplication where it matters most:
- `tile_visible_frames()` — single function for all surfaces ✅
- `sync_scene_visibility()` — single function ✅
- `snap_capture_layout()` / `snap_restore_layout()` — single functions ✅
- `update_local_geometry()` — single match ✅
- `minimize_frame()` / `restore_minimized_frame()` — generic by frame_id ✅

The duplication only lives in the **app-surface lifecycle entry points**: ensure, open, focus, toggle, frame_id. These are surface-specific by nature — they map a keyboard shortcut (F8/F9) to a named surface action.

---

## 5. Recommendation

### **Delay full dedup. Keep named wrappers.**

**Rationale:**
- Only 2 surfaces (Linen, Quil) use this pattern. A 3rd surface (Mesh, Bell, Collar) would justify dedup.
- Named wrappers (`toggle_linen()`, `toggle_quil()`) are clearer at call sites than generic versions with spec lookup.
- Quil's divergent 0xEF fill rect complicates the generic `open_*` function.
- Budget markers per surface are informative in logs — abstraction would either lose them or require string parameters.
- The shell already deduplicates the heavy machinery (tiling, visibility, focus, snap, minimize).

### What to do instead

**Option A (preferred): Add `ensure_app_surface_frame(spec)` preemptively if a 3rd surface appears.**

```rust
// Only when adding Mesh/Bell/Collar:
unsafe fn ensure_app_surface_frame(spec: &AppSurfaceSpec) -> Option<u32> { ... }
// Named wrappers become 2-liners:
unsafe fn ensure_linen_frame() -> Option<u32> {
    ensure_app_surface_frame(&APP_SURFACES[0])
}
```

This gets 80% of the line savings (ensure + open) while keeping named wrappers for focus/toggle/frame_id.

**Option B: Do nothing until a 3rd surface triggers the pattern.**

The current code is correct, tested, and readable. ~280 lines of near-duplicate code for 2 surfaces is acceptable — the logic is simple (frame creation from constants) and isolated (no shared mutable state bugs). The real complexity is in the shared machinery (tiling, visibility, snap), which IS already deduplicated.

**Recommendation: Option B for now. Revisit when adding Mesh/Bell/Collar.**

---

## 6. Next Step (if implementing)

If a 3rd app surface triggers dedup, the implementation would be:

**APP_SURFACE_HELPER_DEDUP_V1** — implement only Phase A + B:
1. Add `ensure_app_surface_frame(spec: &AppSurfaceSpec) -> Option<u32>`
2. Add `open_surface_in_active_scene(spec: &AppSurfaceSpec) -> bool`
3. Convert Linen/Quil ensure wrappers to 2-liners
4. Keep `toggle_*`, `focus_or_open_*`, `*_frame_id` as named wrappers
5. Keep Quil's 0xEF in a thin `open_quil_in_active_scene()` wrapper

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Dedup analysis — recommend delay until 3rd surface | APP_SURFACE_HELPER_DEDUP_PLAN_V1 |
