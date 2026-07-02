# SILK_SAFE_CLOSE_TOMBSTONE_V1

## Status: LANDED

Build: `scripts/entrypoint_build.sh` — `[SEXOS ENTRYPOINT] success`
Date: 2026-05-20
Patch file: `servers/silk-shell/src/main.rs`
Backup: `servers/silk-shell/src/main.rs.bak_safe_close_v1`

---

## What Changed

All edits confined to `close_surface_from_frame_light` in `servers/silk-shell/src/main.rs`.
No kernel, ABI, sex-pdx, or sexdisplay edits. No new ABI/protocol.

### Markers added

| Marker | Location | Condition |
|--------|----------|-----------|
| `[silk.close.request]` | start of `close_surface_from_frame_light` | always on enter |
| `[silk.close.allowed]` | after `is_closeable_surface` → true | disposable/registry/lifecycle surfaces |
| `[silk.close.blocked.core]` | after `is_closeable_surface` → false | OS-protected + non-closeable surfaces |
| `[silk.close.tombstone]` | before lifecycle FSM transitions | just before Closing→Tombstoned→Destroyed |
| `[silk.close.state.clear]` | drag/resize/tab-drag cancel + post-sweep | 4 emit sites |
| `[silk.close.focus.next]` | after `clear_focus_if_dead` | only when frame has remaining live tabs |
| `[silk.close.frame.empty]` | when frame_emptied is Some | alongside existing `[silk.lifecycle.frame.empty.destroy]` |

### Behavior changes

**Focus handoff**: Added `next_focus_sid: Option<u64>` tracking during tab removal. After compaction, if the frame still has tabs, the new `active_tab`'s surface_id is recorded. After `clear_focus_if_dead()`, if that surface is alive and not tombstoned, `try_set_focus` is called directly — prefers frame neighbor over z-order fallback.

**Resizing state clear**: If `INTERACTION` is `Resizing { surface_id }` for the closing surface, `try_transition(Idle)` is called before lifecycle transitions.

**TabDragging state clear**: If `INTERACTION` is `TabDragging { frame_id }` matching the closing surface's frame, `try_transition(Idle)` is called.

**Core app policy**: Non-closeable surfaces (CURSOR, LAUNCHER, STATUS, CLOCK, BELL, SCENE_SETTINGS) emit `[silk.close.blocked.core]` and return false. No kill, no minimize coercion — existing policy preserved.

---

## Proof

```
bash scripts/entrypoint_build.sh
# Output ends with: [SEXOS ENTRYPOINT] success
```

Marker grep:
```
grep "silk\.close\." servers/silk-shell/src/main.rs | grep serial_println
# All 7 markers present at lines: 14904, 14908, 14911, 14931, 14938, 14946, 14964, 15031, 15047, 15058
```

Existing markers preserved:
```
grep -c "silk\.frame\.lights\|silk\.lifecycle\|silk\.frame\.chrome\|app\.lifecycle" \
  servers/silk-shell/src/main.rs
# 114
```

---

## Invariants Preserved

- No tab index >= MAX_TABS_PER_FRAME (existing compaction logic unchanged)
- No frame slot out of bounds (same fixed-array iteration)
- No allocation
- sexdisplay not touched
- No kernel/ABI/sex-pdx edits
- Existing `[silk.lifecycle.*]`, `[silk.frame.lights.*]`, resize/drag/snap markers unchanged
- Drag-to-snap state: covered by post-close `clear_drag_if_dead` + new Resizing/TabDragging clears

---

## Known Unrelated Failure

`sexnet_dns_source3_proof_v1 FAIL` — DNS lane marker issue, predates this patch. Not touched here.

---

## Next Steps

- Boot gate runtime verification if log available
- Pointer-hit path for red frame light already tests close (stage 2 of `maybe_run_frame_lights_pointer_proof`)
- Multitab proof (`maybe_run_silk_lifecycle_multitab_proof`) exercises tab close + neighbor focus
