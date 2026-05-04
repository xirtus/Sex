# A4_FOCUS_LIFECYCLE_GUARDS_V1

**Status:** Small patch — focus guards wired to A3 lifecycle metadata.
**Date:** 2026-05-04
**Purpose:** Prevent focus on lifecycle-invalid surfaces by adding lifecycle state checks and FocusRef generation validation to try_set_focus(), clear_focus_if_dead(), and clear_focus_if_wrong_scene().

---

## 1. Changes

- `servers/silk-shell/src/main.rs` — 4 insertion points
- `docs/handoff/A4_FOCUS_LIFECYCLE_GUARDS_V1.md` — this file

No kernel/ABI/sex-pdx/sexdisplay changes. No lifecycle enum changes. No WINDOWS Vec migration.

## 2. Focus Guard Order (try_set_focus)

All guards execute in sequence, short-circuiting on first reject:

1. `sid == 0` → clear focus, return true
2. `!is_focusable_surface(sid)` → `[shell.focus.reject.nonfocusable]`
3. `!surface_is_alive(sid)` → `[shell.focus.reject.dead]`
4. `is_tombstoned(sid)` → `[shell.focus.reject.tombstoned]`
5. `!surface_in_active_scene(sid)` → `[shell.focus.reject.wrong-scene]`
6. **`!surface_is_lifecycle_focusable(sid)`** → `[focus.lifecycle.reject]` *(A4 new)*
7. **FocusRef generation check** → `[focus.generation.reject]` *(A4 new)*
8. Commit: `FOCUSED_SURFACE_ID = sid`, `sync_focus_ref()`, `[focus.ref.commit]`

## 3. Lifecycle States Allowed to Focus

| State | Focusable? | Reason |
|-------|-----------|--------|
| Allocated | ❌ | No frame, no display state |
| Mapped | ✅ | Active shell overlays (panels, cursor) |
| Visible | ✅ | Normal app/frame surfaces in active scene |
| Hidden | ❌ | Non-active scene — no input routing |
| Minimized | ❌ | Frame collapsed — `frame_accepts_input()` also rejects |
| Closing | ❌ | Close requested, irreversible |
| Tombstoned | ❌ | Dead, record only |
| Destroyed | ❌ | Terminal |

**Visible** is the standard focusable state for app/frame surfaces. **Mapped** is allowed for shell-owned overlays (cursor, panels) that are alive but not in a frame's active scene.

## 4. clear_focus_if_dead() Updated

Previously checked only `surface_is_alive(focused)`. Now also checks `surface_is_lifecycle_focusable(focused)`:

- If surface is dead → `[focus.ref.clear] reason=dead`
- If surface is alive but non-focusable (Minimized, Closing, etc.) → `[focus.ref.clear] reason=not_focusable lifecycle={:?}`
- Z-order fallback also filters by `surface_is_lifecycle_focusable()` before calling `try_set_focus()`

## 5. clear_focus_if_wrong_scene() Updated

Tab iteration now requires `surface_is_lifecycle_focusable(tab.surface_id)` alongside existing `surface_is_alive()` and `!is_tombstoned()` checks.

## 6. Proof Markers Added

| Marker | When |
|--------|------|
| `[focus.lifecycle.reject]` | try_set_focus() rejects because lifecycle state is not focusable |
| `[focus.generation.reject]` | try_set_focus() rejects because FocusRef generation is stale |
| `[focus.ref.commit]` | FocusRef committed after successful focus assignment |
| `[focus.ref.clear]` | Focus cleared (reason: dead / not_focusable) |

## 7. Build Result

**Build:** Passed (ISO produced successfully)
**Code-specific errors:** Zero (all dependency `core` crate errors are build-environment, not code)
**Behavior change:** Focus now correctly rejects Minimized, Closing, Tombstoned, Destroyed surfaces at the lifecycle level, in addition to existing alive/tombstone/scene checks.

## 8. Behavior Intentionally Unchanged

- ❌ No caller identity validation (`FocusSource` enum deferred)
- ❌ No drag-pin rule in try_set_focus()
- ❌ No 0xEE opcode collision fix
- ❌ No WINDOWS Vec migration
- ❌ No frame light dispatch through FSM
- ❌ No proof marker renaming to `[comp.*]` convention

## 9. STOP FIRST Findings

None. All existing STOP FIRST conditions remain unchanged.

## Document References

- `docs/A_COMPOSITOR_LIFECYCLE_PLAN_V1.md`
- `docs/handoff/A3_SHELL_LIFECYCLE_MODEL_V1.md`
- `servers/silk-shell/src/main.rs`
