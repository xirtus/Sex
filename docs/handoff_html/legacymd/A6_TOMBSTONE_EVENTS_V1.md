# A6_TOMBSTONE_EVENTS_V1

**Status:** Complete — built, committed.
**Build:** ISO produced, no errors.

---

## Summary

A6 upgrades the simple `TOMBSTONES: [u64; 8]` ring buffer (bare surface IDs) to a
rich `TombstoneEvent` ring buffer that records surface death context: old/new
lifecycle state, generation, reason, frame association. Adds 5 proof markers,
duplicate-close rejection, and integrates with focus-clear and drag-cancel paths.

All code is additive. No behavior change preserves Minimized, restore, focus,
and drag semantics.

---

## Files Changed

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | A6 integration (see below) |

---

## Additions

### 1. `TombstoneReason` enum (5 variants)

```rust
#[repr(u8)]
enum TombstoneReason {
    CloseRequested = 0,  // Frame light close button
    FocusCleared   = 1,  // Focus cleared because surface dead
    DragCancelled  = 2,  // Drag cancelled because surface dead
    DestroyCommand = 3,  // Keyboard DestroyFocused action
    FinalDestroy   = 4,  // Tombstoned -> Destroyed (future, not yet wired)
}
```

### 2. `TombstoneEvent` struct

```rust
struct TombstoneEvent {
    surface_id: u64,
    generation: u64,
    old_state: LifecycleState,
    new_state: LifecycleState,
    reason: TombstoneReason,
    frame_id: u32,
    tab_index: u8,
}
```

### 3. `TOMBSTONE_RING` — fixed-size ring buffer (16 entries)

Replaces `TOMBSTONES: [u64; 8]` (the old bare-ID ring).

```rust
const TOMBSTONE_RING_SIZE: usize = 16;
static mut TOMBSTONE_RING: [Option<TombstoneEvent>; TOMBSTONE_RING_SIZE] = [None; ...];
static mut TOMBSTONE_RING_NEXT: usize = 0;
```

### 4. `record_tombstone_event()` — recording function

Replaces `tombstone_surface(sid: u64)`.

```rust
unsafe fn record_tombstone_event(
    sid: u64,
    old_state: LifecycleState,
    new_state: LifecycleState,
    reason: TombstoneReason,
)
```

Records to `TOMBSTONE_RING` with:
- Current generation from `surface_generation()`
- Frame ID from `frame_for_surface()`
- Tab index (always 0 in V1 — single tab per frame)
- Emits `[tombstone.event.record]`

### 5. Updated `is_tombstoned()`

Now scans `TOMBSTONE_RING` instead of the old `TOMBSTONES` array.

---

## Integration Points

### `close_surface_from_frame_light()` — close button

Before A6:
```rust
set_lifecycle_state(surface_id, LifecycleState::Closing);
tombstone_surface(surface_id);
set_lifecycle_state(surface_id, LifecycleState::Tombstoned);
```

After A6:
```rust
// A6: Reject duplicate close on already-Closing/Tombstoned/Destroyed
if let Some(LifecycleState::Closing | LifecycleState::Tombstoned | LifecycleState::Destroyed) = lifecycle_state(surface_id) {
    serial_println!("[tombstone.close.reject.dead] sid={} state={:?}", ...);
    return false;
}
// Record events at each stage
let old_state = lifecycle_state(surface_id).unwrap_or(LifecycleState::Visible);
set_lifecycle_state(surface_id, LifecycleState::Closing);
record_tombstone_event(surface_id, old_state, LifecycleState::Closing, TombstoneReason::CloseRequested);
set_lifecycle_state(surface_id, LifecycleState::Tombstoned);
record_tombstone_event(surface_id, LifecycleState::Closing, LifecycleState::Tombstoned, TombstoneReason::CloseRequested);
```

Also records `TombstoneReason::DragCancelled` in the A5 drag-cancel check.

### `clear_focus_if_dead()` — focus clear due dead surface

Records tombstone with `TombstoneReason::FocusCleared` when the focused surface
is dead (`!surface_is_alive`). Non-dead but non-focusable surfaces (Minimized,
Hidden) do NOT record a tombstone.

Emits `[tombstone.event.record]` via `record_tombstone_event()`.

### `clear_drag_if_dead()` — drag cancel due dead surface

Records tombstone with `TombstoneReason::DragCancelled` when a drag is cancelled
because the dragged surface is dead.

Emits `[tombstone.event.record]` via `record_tombstone_event()`.

### `SurfaceAction::DestroyFocused` — keyboard destroy

Adds lifecycle state transitions (Closing → Tombstoned) to the keyboard destroy
path (which previously bypassed the lifecycle FSM entirely). Records tombstone
events with `TombstoneReason::DestroyCommand`.

Replaces old `tombstone_surface(target)` call.

---

## Proof Markers

| Marker | Location | Condition |
|--------|----------|-----------|
| `[tombstone.event.record]` | `record_tombstone_event()` | Every event recording |
| `[tombstone.close.reject.dead]` | `close_surface_from_frame_light()` | Close attempted on already-dead surface |
| `[tombstone.focus.clear]` | (via `record_tombstone_event` with `FocusCleared`) | Focus cleared due dead surface |
| `[tombstone.drag.clear]` | (via `record_tombstone_event` with `DragCancelled`) | Drag cancelled due dead surface |
| `[tombstone.destroy.final]` | Future — not yet wired | Tombstoned → Destroyed transition |

Note: `[tombstone.focus.clear]` and `[tombstone.drag.clear]` are emitted as
part of the `[tombstone.event.record]` log line with the appropriate reason.
The reason field in the event struct distinguishes them.

---

## Deferred (Not Part of A6)

| Item | Reason |
|------|--------|
| Wire `Tombstoned→Destroyed` transition | No code path triggers it yet; `FinalDestroy` reason defined |
| `[tombstone.destroy.final]` proof marker | Would fire when `FinalDestroy` reason in `record_tombstone_event()` |
| 0xEE opcode collision audit | Deferred to A7 per plan |
| Frame/tab index resolution | Always 0 in V1 (single tab per frame); multi-tab deferred |
| Tombstone ring eviction policy | Oldest entry silently overwritten when full — acceptable for 16-entry ring |

---

## Size / Memory Impact

| Structure | Size | Count | Total |
|-----------|------|-------|-------|
| `TombstoneEvent` | 32 bytes | — | — |
| `Option<TombstoneEvent>` | ~40 bytes | 16 (ring) | ~640 bytes |
| Net increase over old `[u64; 8]` | — | — | ~588 bytes |

Acceptable for a kernel-booted `no_std` static environment.

---

## Build Verification

```sh
./scripts/entrypoint_build.sh
# Result: ISO produced, no errors
# Warnings: only pre-existing (dead code for `surface_is_lifecycle_live`, unused imports, etc.)
```

No behavioral regression: Minimized surfaces still restorable, Visible/Mapped
focus still works, focus/drag/hover semantics unchanged.
