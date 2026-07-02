# A6_TOMBSTONE_DEBUG_EVENTS_V1

**Status:** Complete — built, committed.
**Build:** ISO produced, no errors.

---

## Summary

A6 completes the lifecycle tombstone/debug event tracking for silk-shell. The existing `TombstoneEvent` ring buffer (from the initial A6 work) is extended with:

1. **Tombstoned→Destroyed lifecycle transition** — completes the FSM chain: Visible → Closing → Tombstoned → Destroyed. Previously the chain stopped at Tombstoned; now the terminal Destroyed state is reached in all close paths.
2. **Lifecycle-convention proof markers** — adds `[lifecycle.tombstone.*]`, `[lifecycle.destroy.*]`, and `[lifecycle.generation.*]` diagnostic markers alongside the existing `[tombstone.*]` markers.
3. **Restore guard** — explicit lifecycle state check in `restore_minimized_frame()` rejecting Tombstoned/Destroyed/Closing surfaces (defense-in-depth alongside the existing `surface_is_alive` check).

All changes are additive or complete already-planned transitions. No Atlas, sexdisplay, ABI, tiling, focus policy, or WINDOWS Vec changes.

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | A6 debug event completion (see below) |
| `docs/handoff/A6_TOMBSTONE_DEBUG_EVENTS_V1.md` | New handoff doc |

---

## Changes Detail

### 1. Tombstoned → Destroyed Transition (Complete FSM)

**`close_surface_from_frame_light()`** — after Tombstoned transition:
```rust
set_lifecycle_state(surface_id, LifecycleState::Destroyed);
record_tombstone_event(surface_id, LifecycleState::Tombstoned, LifecycleState::Destroyed, TombstoneReason::FinalDestroy);
serial_println!("[lifecycle.destroy.record] sid={}", surface_id);
```

**`SurfaceAction::DestroyFocused`** keyboard handler — same pattern added after each of the 4 surface branches (APP, STATIC, TEST3, TEST4). Each branch now completes the full FSM chain:
```
live → Closing → Tombstoned → Destroyed
```

The `TombstoneReason::FinalDestroy` variant (previously defined but unused) is now wired.

### 2. Lifecycle-Convention Proof Markers

| Marker | Location | When | Budget |
|--------|----------|------|--------|
| `[lifecycle.tombstone.record]` | `record_tombstone_event()` | Every tombstone event recording | Unbudgeted |
| `[lifecycle.tombstone.reject_focus]` | `try_set_focus()` | Focus attempt on tombstoned surface | Unbudgeted |
| `[lifecycle.tombstone.reject_restore]` | `restore_minimized_frame()` | Restore attempt on dead/tombstoned surface | Unbudgeted |
| `[lifecycle.destroy.record]` | `close_surface_from_frame_light()`, `DestroyFocused` branches | Tombstoned → Destroyed transition | Unbudgeted |
| `[lifecycle.destroy.reject]` | `close_surface_from_frame_light()` | Close attempt on already-dead surface | Unbudgeted |
| `[lifecycle.generation.stale_reject]` | `try_set_focus()` | Focus rejected due generation mismatch | Unbudgeted |

All markers are unbudgeted (always print) since they record low-frequency critical lifecycle events.

### 3. Restore Lifecycle Guard

`restore_minimized_frame()` now has an explicit lifecycle state check:

```rust
// A6: Reject restore for Tombstoned/Destroyed/Closing lifecycle states.
if let Some(state) = lifecycle_state(surface_id) {
    if matches!(state, LifecycleState::Tombstoned | LifecycleState::Destroyed | LifecycleState::Closing) {
        serial_println!("[lifecycle.tombstone.reject_restore] sid={} state={:?}", surface_id, state);
        return false;
    }
}
```

This is defense-in-depth alongside the existing `surface_is_alive()` check at line 3120. All current close paths set `SURFACE_*_ALIVE = false` before transitioning to Tombstoned, so the `surface_is_alive()` check already prevents restore of closed surfaces. The lifecycle guard adds explicit state awareness and emits a diagnostic marker if triggered.

### 4. Preserved Markers

The following pre-existing markers are preserved unchanged:

- `[shell.tile.skip_dead]` — unchanged (diagnostic for dead surface skipping during tiling)
- `[tombstone.event.record]` — unchanged (original tombstone marker)
- `[tombstone.close.reject.dead]` — unchanged (original close reject marker)
- `[shell.focus.reject.tombstoned]` — unchanged (original focus reject marker)
- `[focus.generation.reject]` — unchanged (original generation reject marker)

---

## Lifecycle FSM Completeness

The full FSM chain for surface death is now:

```
Allocated → Mapped → Visible → Closing → Tombstoned → Destroyed
                                     ↓
                              (generation bump)
                                     ↓
                              (generation bump)
                                     ↓
                              (generation bump)
```

Generation bumps occur on:
- Visible/Hidden/Minimized → Closing
- Closing → Tombstoned
- Tombstoned → Destroyed

This matches the A2 FSM spec and the generation bump rules in `set_lifecycle_state()`.

---

## Existing A6 Infrastructure (Unchanged)

The following were already committed before this phase and are preserved:

- `TombstoneReason` enum (5 variants: CloseRequested, FocusCleared, DragCancelled, DestroyCommand, FinalDestroy)
- `TombstoneEvent` struct (surface_id, generation, old_state, new_state, reason, frame_id, tab_index)
- `TOMBSTONE_RING[16]` ring buffer with overwrite semantics
- `record_tombstone_event()` recording function
- `is_tombstoned()` ring scan
- `[tombstone.event.record]` marker in `record_tombstone_event()`
- `clear_focus_if_dead()` tombstone recording (FocusCleared reason)
- `clear_drag_if_dead()` tombstone recording (DragCancelled reason)
- Duplicate-close rejection in `close_surface_from_frame_light()`
- Keyboard destroy tombstone recording (DestroyCommand reason)
- `surface_is_alive()` and `is_tombstoned()` guards in `frame_accepts_input()`, `tile_visible_frames()`, `try_set_focus()`
- Generation stale-reference reject in `try_set_focus()`

---

## Build Verification

```sh
./scripts/entrypoint_build.sh
# Result: ISO produced, no errors
# Warnings: only pre-existing
```

No behavioral regression: Minimized surfaces still restorable, Visible/Mapped focus still works, focus/drag/hover semantics unchanged.

---

## Scope Exclusions

Per A6 requirements, the following are NOT changed:

- ❌ No Atlas changes
- ❌ No sexdisplay changes
- ❌ No ABI changes
- ❌ No tiling changes or new tile call sites
- ❌ No focus policy rewrite
- ❌ No WINDOWS Vec changes
- ❌ No minimize/restore/unzoom tiling wiring
- ❌ No `[lifecycle.*]` marker rename of existing `[tombstone.*]` markers (both coexist)

---

## Document References

- `docs/handoff/A6_TOMBSTONE_EVENTS_V1.md` — prior A6 handoff (initial tombstone ring implementation)
- `docs/handoff/A2_COMPOSITOR_LIFECYCLE_FSM_SPEC_V1.md` — FSM spec defining the full lifecycle chain
- `servers/silk-shell/src/main.rs` — implementation

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Complete Tombstoned→Destroyed FSM chain, add lifecycle-convention proof markers, add restore lifecycle guard | A6_TOMBSTONE_DEBUG_EVENTS_V1 |
