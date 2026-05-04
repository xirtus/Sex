# A3_SHELL_LIFECYCLE_MODEL_V1

**Status:** Handoff/spec only. Additive metadata, no behavior changed.
**Date:** 2026-05-04
**Purpose:** Document the A3 lifecycle state tracking added to silk-shell. Metadata-first approach — A4 will harden focus using the new model.

---

## 1. Executive Summary

A3 adds explicit lifecycle metadata to silk-shell's existing surface/frame model without changing any user-visible behavior. Eight lifecycle states are tracked per surface via a static array (LIFECYCLE_TABLE), alongside a monotonic LifecycleGeneration counter for stale reference detection. A FocusRef struct shadows the existing FOCUSED_SURFACE_ID for A4 readiness. No booleans were removed, no behavior was changed, and the existing WINDOWS Vec was preserved.

**Core principle:** Metadata first, policy second. A3 only adds the tracking infrastructure. A4 will harden focus guards, A5 will wire frame lights through the FSM.

---

## 2. Files Changed

- `servers/silk-shell/src/main.rs` (5112 → 5556 lines)
- `docs/handoff/A3_SHELL_LIFECYCLE_MODEL_V1.md` (this file)

No other files touched. No kernel/ABI/sex-pdx edits. No sexdisplay changes.

---

## 3. LifecycleState Enum Added

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum LifecycleState {
    Allocated = 0,   // SurfaceId reserved, no frame mapped
    Mapped = 1,      // Surface attached to Frame, may not be visible
    Visible = 2,     // Active scene, not minimized, receives input
    Hidden = 3,      // Non-active scene, no input routing
    Minimized = 4,   // Frame collapsed, surface hidden
    Closing = 5,     // Close requested, irreversible
    Tombstoned = 6,  // Dead but record exists, no focus
    Destroyed = 7,   // Terminal, eligible for reuse with gen safety
}
```

**Location:** Line 135-153, after APP_SURFACES registry.

---

## 4. FocusRef Shadow Added

```rust
#[derive(Debug, Clone, Copy)]
struct FocusRef {
    surface_id: u64,
    generation: u64,
}

static mut FOCUSED_SURFACE: Option<FocusRef> = None;
```

FOCUSED_SURFACE is a shadow of the existing FOCUSED_SURFACE_ID: u64. It is updated in parallel via `sync_focus_ref()` called after all state changes in the main loop and at boot. Does not change focus behavior.

**Location:** Lines 157-185.

---

## 5. LifecycleGeneration Counter

```rust
static mut LIFECYCLE_GENERATION: u64 = 1;
```

- Starts at 1. 0 reserved for "no surface".
- Incremented on transitions that invalidate stale references:
  - Any live state (Visible/Hidden/Minimized) entering Closing
  - Closing → Tombstoned
  - Tombstoned → Destroyed
  - Any state → Destroyed (direct destroy path)
- Wraparound checked at each increment. If generation would wrap to 0, the counter saturates and a `[lifecycle.generation.bump.wrap]` marker is emitted.

**Location:** Lines 181-183.

---

## 6. Surface Lifecycle Storage

```rust
const LIFECYCLE_MAX_SURFACES: usize = 32;
static mut LIFECYCLE_TABLE: [Option<(u64, SurfaceLifecycle)>; LIFECYCLE_MAX_SURFACES] = [None; ...];
```

Linear search array (max 32 entries), preserving the existing pattern of hardcoded SurfaceIds. No dynamic allocation. Matches the existing codebase convention of static arrays for state tracking.

**Location:** Lines 175-179.

---

## 7. Helper Functions Added

| Function | Signature | Purpose |
|----------|-----------|---------|
| `lifecycle_register` | `(sid, initial_state) -> bool` | Register surface at boot |
| `lifecycle_state` | `(sid) -> Option<LifecycleState>` | Lookup current state |
| `set_lifecycle_state` | `(sid, next) -> bool` | Update state, bump generation |
| `surface_generation` | `(sid) -> Option<u64>` | Lookup current generation |
| `bump_surface_generation` | `(sid) -> bool` | Force generation bump |
| `make_focus_ref` | `(sid) -> Option<FocusRef>` | Create FocusRef from surface_id |
| `focus_ref_is_current` | `(&FocusRef) -> bool` | Validate FocusRef is not stale |
| `sync_focus_ref` | `() -> ()` | Derive FOCUSED_SURFACE from FOCUSED_SURFACE_ID |
| `surface_is_lifecycle_live` | `(sid) -> bool` | Check if surface is in live state |
| `surface_is_lifecycle_focusable` | `(sid) -> bool` | Check if surface can receive focus |
| `lifecycle_init_all` | `() -> ()` | Register all known surfaces at boot |

**Location:** Lines 1495-1675 (after clear_drag_if_wrong_scene, before is_shell_surface).

---

## 8. Generation Bump Rules Implemented

Generation bumps in `set_lifecycle_state()` when:

```
Visible/Hidden/Minimized → Closing    (stale ref dies when close begins)
Closing → Tombstoned                   (final close transition)
Tombstoned → Destroyed                 (reclamation)
Any → Destroyed                        (direct destroy path)
```

This matches the A2 spec corrected per review: generation bumps on entering Closing, not just on Closing→Tombstoned.

---

## 9. Initial Lifecycle States at Boot

| Surface ID | Surface | Initial State |
|-----------|---------|---------------|
| 100 | APP | Visible |
| 101 | STATIC | Visible |
| 102 | TEST3 | Visible |
| 103 | TEST4 | Visible |
| 200 | LINEN | Visible |
| 201 | QUIL | Visible |
| 0x90 | CURSOR | Mapped |
| 0x92 | LAUNCHER | Allocated |
| 0x93 | STATUS | Allocated |
| 0x94 | CLOCK | Allocated |
| 0x95 | BELL | Allocated |
| 0x96 | SCENE_SETTINGS | Allocated |
| 0x97 | ATLAS_OVERLAY | Allocated |

**Initialization call:** `lifecycle_init_all()` in `_start()` after frame init, before snapshot capture.

---

## 10. Lifecycle Tracking Integration Points

| Operation | Lifecycle Transition | Location |
|-----------|---------------------|----------|
| Surface close | Visible/Hidden/Minimized → Closing → Tombstoned | `close_surface_from_frame_light()` |
| Frame minimize | Visible/Hidden → Minimized | `minimize_frame()` |
| Frame restore | Minimized → Visible | `restore_minimized_frame()` |
| Panel open | Allocated → Mapped | `toggle_os_panel()`, `toggle_scene_settings_panel()` |
| Panel close | Mapped → Allocated | `toggle_os_panel()`, `toggle_scene_settings_panel()` |
| Atlas enter | Allocated → Mapped | `atlas_render_stub()` |
| Atlas exit | Mapped → Allocated | `atlas_clear_stub()`, click handler, keyboard handler |

All tracking is additive — no behavioral boolean or flag was modified.

---

## 11. Proof Markers Added

| Marker | When |
|--------|------|
| `[lifecycle.state.init]` | Boot initialization complete |
| `[lifecycle.transition.allow]` | State transition succeeded |
| `[lifecycle.transition.reject]` | State transition failed (unknown surface) |
| `[lifecycle.generation.bump]` | Generation incremented |
| `[lifecycle.generation.bump.wrap]` | Generation wraparound detected (saturated) |
| `[lifecycle.focusref.make]` | FocusRef created |
| `[lifecycle.focusref.reject]` | FocusRef validation failed |

---

## 12. Build Result

- **Build:** Passed (`./scripts/entrypoint_build.sh` produced ISO successfully)
- **Errors in changes:** Zero (all `LifecycleState` and `set_lifecycle_state` resolution errors fixed)
- **Pre-existing warnings:** 11 warnings (unnecessary `unsafe` blocks in event loop — pre-existing)
- **Kernel/ABI/sex-pdx changes:** None
- **sexdisplay changes:** None

---

## 13. Behavior Intentionally Unchanged

The following A doc features are NOT implemented in A3 (by design):

- ❌ No Closing→Tombstoned→Destroyed behavioral chain (close still jumps directly to dead)
- ❌ No 0xEE opcode collision fix
- ❌ No WINDOWS Vec migration to static arrays
- ❌ No A4 focus guard rewrite (caller identity, generation check, minimized check, drag-pin)
- ❌ No A5 frame light dispatch through FSM
- ❌ No A6 tombstone generation tracking
- ❌ No drag-cancellation-before-close guard
- ❌ No proof marker renaming to `[comp.*]` convention

All existing behavior is preserved. The lifecycle metadata is purely observational.

---

## 14. Blockers for A4

1. **Caller identity validation:** `try_set_focus()` has no mechanism to distinguish shell-internal focus changes from PD-originated requests. A4 must design `FocusSource` enum and validation rules.
2. **Generation check:** `try_set_focus()` does not validate FocusRef generation. The helper `focus_ref_is_current()` exists and is ready for A4 to wire in.
3. **Minimized check:** `try_set_focus()` does not call `frame_accepts_input()`. `surface_is_lifecycle_focusable()` exists for A4 to use.
4. **Drag-pin rule:** `try_set_focus()` does not check `InteractionState::Dragging`. A4 must add this check.
5. **clear_focus_if_dead() z-order:** Still uses hardcoded z-order `[QUIL, LINEN, TEST4, TEST3, STATIC, APP]`. A4 should derive z-order from frame state.

## 15. Blockers for A5

1. **Frame light dispatch:** Red/yellow/green lights still bypass FSM. `close_surface_from_frame_light()` does not check drag before close.
2. **Drag-before guard:** `close_surface_from_frame_light()` and `toggle_zoom_frame()` do not check for active drag before proceeding. "Cancel drag before transition" invariant not yet implemented.

## 16. STOP FIRST Findings

- No STOP FIRST violations found in A3 changes.
- All existing STOP FIRST conditions remain unchanged.
- The `WINDOWS Vec` remains heap-backed (A2 decision: preserve temporarily, address in A5).

## 17. Ready for A4?

**Yes.** A3 provides all the metadata plumbing A4 needs:
- `LifecycleState` enum for state queries
- `FocusRef` + `focus_ref_is_current()` for generation safety
- `surface_is_lifecycle_focusable()` for focus eligibility
- `surface_is_lifecycle_live()` for liveness checks
- `sync_focus_ref()` for FocusRef currency

A4 should wire these into `try_set_focus()` guards and update `clear_focus_if_dead()`.

## Document References

- `docs/A_COMPOSITOR_LIFECYCLE_PLAN_V1.md` — parent plan doc
- `docs/handoff/A1_COMPOSITOR_LIFECYCLE_AUDIT_V1.md` — audit findings
- `docs/handoff/A2_COMPOSITOR_LIFECYCLE_FSM_SPEC_V1.md` — FSM spec
- `servers/silk-shell/src/main.rs` — implementation (additive metadata only)
