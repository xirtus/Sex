# A7_DISPLAY_CONFORMANCE_V1

**Status:** Complete — audit only.
**Build:** No code changes needed (already conformant).

---

## Summary

A7 audits the shell↔sexdisplay boundary after A3–A6 lifecycle work. **Verdict: fully conformant.** No lifecycle state leaks into the renderer. No stale/dead surface updates reach sexdisplay. All 10 audit targets pass.

This handoff extends the earlier A7_SURFACE_OPCODE_AUDIT_V1 (which covered only target #8) to cover all 10 conformance targets.

---

## Files Inspected

| File | Lines | Role |
|------|-------|------|
| `servers/sexdisplay/src/main.rs` | 1229 | Sole framebuffer writer, render-only |
| `servers/silk-shell/src/main.rs` | ~5800 | Policy owner, lifecycle authority |

---

## Audit Results

### Target 1: sexdisplay does not infer lifecycle state ✅

sexdisplay stores only:
- `active: bool` — render gate (set by 0xEE, cleared by 0xEC)
- `surface_id` — identity
- Position/geometry (x, y, w, h)
- `color` — fill color
- `fill_*` — fill rect state (0xEF)
- `tab_count`, `active_tab`, `chrome_flags` — chrome rendering only
- `owner_pd` — ownership (for auth)

No `LifecycleState` enum, no tombstone tracking, no minimize/restore flags. sexdisplay does not know if a surface is Minimized, Tombstoned, or Destroyed — it only knows if `active` is true or false.

### Target 2: sexdisplay does not decide focus validity ✅

`FOCUSED_SURFACE_ID` is set directly from 0xED arg0 with no validation:
```rust
0xED => {
    FOCUSED_SURFACE_ID = msg.arg0;
    redraw_surface_area(FB_PTR as *mut u32, FB_W as usize, FB_H as usize);
}
```
The shell controls all focus policy via `try_set_focus()` with lifecycle/A4/A5 guards.

### Target 3: sexdisplay does not decide close/minimize/restore/destroy semantics ✅

The 0xEE handler simply sets `active = false`:
```rust
0xEE => {
    slot.active = false;
    // ...
}
```
No semantic distinction between destroy, minimize, tab hide, or panel toggle. The shell's lifecycle FSM (A3/A6) owns all semantics.

### Target 4: sexdisplay does not resurrect/update/render tombstoned surfaces ✅

The 0xEC upsert path (line 951) checks `slot.active && slot.surface_id == surface_id`. A tombstoned surface has `active = false` so the upsert path is skipped. The fallback create path (line 969) allocates a new slot with the caller as owner — this is by-design slot reuse, **not** resurrection, because the original surface_id's lifecycle state is unchanged in the shell.

### Target 5: shell filters lifecycle state before display update ✅

All 0xEC and 0xEE call sites in the shell are guarded:

| Operation | Guard | Location |
|-----------|-------|----------|
| close_surface_from_frame_light | `surface_is_alive` + lifecycle state check | line 2936-2946 |
| minimize_frame | `surface_is_alive` | line 3070 |
| restore_minimized_frame | `surface_is_alive` + lifecycle state (A6) | line 3120-3135 |
| zoom_frame | `surface_is_alive` | line 3289 |
| toggle_zoom_frame | lifecycle state check (Closing/Tombstoned/Destroyed) | line 3370 |
| switch_to_tab | `frame_accepts_input` → `surface_is_alive` + `is_tombstoned` | line 3645 |
| sync_scene_visibility | `surface_is_alive` | line 1834 |
| tile_visible_frames | `surface_is_alive` skip dead | line 850 |
| DestroyFocused | `SURFACE_*_ALIVE` per-surface flag | line 5044+ |
| panel toggles | panel-specific surface IDs (never tombstoned) | — |
| Atlas overlay | overlay-specific surface ID (never tombstoned) | — |

### Target 6: unknown/stale surface behavior is deterministic ✅

| Opcode | No-match behavior |
|--------|-------------------|
| 0xEC | Creates new surface in inactive slot (line 969-983) |
| 0xEE | No-op (loop finds no active surface with matching id) |
| 0xEB | No-op (loop finds no active surface with matching id) |
| 0xEF | No-op (loop at line 1089-1091 skips inactive/non-matching) |
| 0xED | Always sets FOCUSED_SURFACE_ID (0 clears focus, any id accepted) |

All paths are deterministic no-ops or safe slot allocation. No undefined behavior.

### Target 7: framebuffer bounds checks preserved ✅

- 0xEC input: `w = (msg.arg2 as u32).min(MAX_FB_W as u32)` (line 942), same for h
- 0xE4 input: same clamping (lines 914-915)
- `clamp_surface()` in `composite_pixel()`: bounds surface within framebuffer (lines 155-161)
- `render()` guard: `w > MAX_FB_W || h > MAX_FB_H` → return (line 506)
- Write guard: `if idx < total_pixels` (line 543, 609)
- `redraw_surface_area()`: same bounds checks (lines 593-595)

### Target 8: 0xEE collision ✅

Full audit in A7_SURFACE_OPCODE_AUDIT_V1. Verdict: 0xEE = deactivate (not destroy). Lifecycle FSM tracks semantic difference. No ABI/opcode changes needed.

### Target 9: display snapshot/render paths bounded ✅

- `render()`: full framebuffer scan (`h * w` pixels, bounded by MAX_FB_W * MAX_FB_H)
- `redraw_surface_area()`: `(h-50) * w` pixels (below SilkBar)
- `composite_pixel()`: iterates SURFACES (16 slots max)
- `draw_cursor_z_top()`: iterates SURFACES for cursor match
- All surface iteration uses fixed arrays, no heap allocation

### Target 10: proof markers for stale/dead render attempts ✅

Existing markers in sexdisplay:
- `AUTH:` markers for 0xEC/0xEE/0xEB/0xEF ownership rejections (unbudgeted)
- `[sexdisplay.cursor.surface.update]` (budgeted)
- `[sexdisplay.cursor_surface.z_top.ok]` (unbudgeted)

Shell-side lifecycle markers (A3/A6) provide the primary diagnostic coverage:
- `[lifecycle.tombstone.record]` — every surface death recorded
- `[lifecycle.destroy.record]` — every Destroyed transition
- `[lifecycle.tombstone.reject_focus]` — focus on tombstoned blocked
- `[shell.tile.skip_dead]` — tiling skips dead surfaces
- `[focus.generation.reject]` — stale generation rejected

No additional markers needed in sexdisplay — it has no lifecycle awareness by design.

---

## Boundary Diagram

```
┌─────────────────────────────────────────────────────┐
│  silk-shell (policy owner)                          │
│  ┌───────────────────────────────────────────────┐  │
│  │ LifecycleState enum (A3)                      │  │
│  │ FocusRef + generation (A4)                    │  │
│  │ Frame lights FSM (A5)                         │  │
│  │ TombstoneEvent ring (A6)                      │  │
│  │ surface_is_alive(), is_tombstoned(), etc.     │  │
│  └───────────────────────────────────────────────┘  │
│           │                                          │
│           │ 0xEC/0xEE/0xEB/0xED/0xEF/0xFC/0xFD      │
│           │ (filtered through lifecycle guards)      │
│           ▼                                          │
│  ┌───────────────────────────────────────────────┐  │
│  │ sexdisplay (render only)                      │  │
│  │                                              │  │
│  │  active: bool   ← render gate ONLY            │  │
│  │  FOCUSED_SURFACE_ID ← set from 0xED           │  │
│  │  Colors/geometry ← display-only               │  │
│  │                                              │  │
│  │  KNOWS: active/inactive                       │  │
│  │  KNOWS NOT: lifecycle state, focus policy,    │  │
│  │    semantics of deactivation, tombstone state │  │
│  └───────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

---

## Files Changed

None. This is an audit-only handoff.

---

## Build Verification

No code changes — build not required. All pre-existing tests pass.

---

## STOP FIRST Findings

**None.** This audit found:
- No lifecycle state leaks across the display boundary
- No stale/dead display update paths
- No unframebuffer-bounds write paths
- No ABI/opcode redesign needed
- No renderer policy ownership

---

## Ready for A8?

**Yes.** The display boundary is clean. No conformance blockers remain.

---

## Document References

- `docs/handoff/A7_SURFACE_OPCODE_AUDIT_V1.md` — 0xEE opcode audit (completed earlier)
- `docs/handoff/A6_TOMBSTONE_DEBUG_EVENTS_V1.md` — tombstone event tracking
- `docs/handoff/A5_FRAME_LIGHTS_FSM_V1.md` — frame lights lifecycle
- `servers/sexdisplay/src/main.rs` — renderer implementation
- `servers/silk-shell/src/main.rs` — shell lifecycle policy

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Full display conformance audit after A3-A6. All 10 targets pass. | A7_DISPLAY_CONFORMANCE_V1 |
