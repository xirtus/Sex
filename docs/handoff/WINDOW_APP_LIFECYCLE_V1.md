# WINDOW_APP_LIFECYCLE_V1

**Status:** ALL 6 PHASES COMPLETE. Build passes for silk-shell and sexdisplay.
**Date:** 2026-05-20
**Correction 2026-05-20:** `[silk.lifecycle.surface.live]` was originally placed inside
`surface_is_lifecycle_live()` — a function with zero call sites (dead code). Marker
relocated to `lifecycle_init_all()` boot path. See §3 Phase 2 notes.
**Prompt:** `WINDOW_APP_LIFECYCLE_AUTOPILOT_V1`
**Build:** `cargo check` passes (silk-shell, sexdisplay)

---

## 1. CURRENT REALITY

### 1.1 Surface/Window ID Allocation

All surface IDs are static `u64` constants in `servers/silk-shell/src/main.rs`:

| Constant | Value | Purpose |
|----------|-------|---------|
| `SURFACE_ID_APP` | 100 | Primary app surface |
| `SURFACE_ID_STATIC` | 101 | Static/Test2 app surface |
| `SURFACE_ID_TEST3` | 102 | Test3 app surface |
| `SURFACE_ID_TEST4` | 103 | Test4 app surface |
| `SURFACE_ID_LINEN` | 200 | Linen object browser |
| `SURFACE_ID_QUIL` | 201 | Quil editor |
| `SURFACE_ID_MESH` | 202 | Mesh workspace |
| `SURFACE_ID_COLLAR` | 203 | Collar security |
| `SURFACE_ID_BELL_PLACEHOLDER` | 204 | Bell notifications |
| `SURFACE_ID_BROWSER` | 205 | WebStub/Browser |
| `SURFACE_ID_CURSOR` | 0x90 (144) | OS-owned cursor |
| `SURFACE_ID_LAUNCHER` | 0x92 (146) | Launcher overlay |
| `SURFACE_ID_STATUS` | 0x93 (147) | Status panel |
| `SURFACE_ID_CLOCK` | 0x94 (148) | Clock panel |
| `SURFACE_ID_BELL` | 0x95 (149) | Bell panel |
| `SURFACE_ID_SCENE_SETTINGS` | 0x96 (150) | Scene settings |
| `SURFACE_ID_ATLAS_OVERLAY` | 0x97 (151) | Atlas overview |
| `SURFACE_ID_COMMAND_PALETTE` | 0x98 (152) | Command palette |
| `SURFACE_ID_SPINDLE` | 0x99 (153) | Spindle terminal |

SURFACE_XXX_ALIVE booleans (100-103) track app surface aliveness. Generation-based lifecycle tracking uses `LIFECYCLE_TABLE` (32 entries, static).

### 1.2 App Launch/Open Path

Apps are hardcoded surfaces at boot. No dynamic app spawning in V1. Launch toggles surface visibility via SilkBar interaction or keyboard shortcuts. `try_set_focus()` handles focus transitions.

### 1.3 Focus Path

- `FOCUSED_SURFACE_ID` (static mut u64) -- current focused surface
- `FOCUSED_SURFACE` (Option<FocusRef>) -- generation-aware shadow, synced via `sync_focus_ref()`
- `try_set_focus(sid)` -- gate: alive check, lifecycle check, scene check, frame check
- `clear_focus_if_dead()` -- clears focus to next live surface in z-order
- `clear_drag_if_dead()` -- cancels drag if target surface dead
- `clear_hover_if_dead()` -- clears hover if active surface dead/tombstoned

### 1.4 Tab/Frame Model (IMPLEMENTED)

```rust
struct ShellFrame {
    frame_id: u32,
    active_tab: u8,
    tab_count: u8,
    tabs: [Option<ShellTab>; 8],
    scene_id: u8,
    flags: u32,
    normal_x/y/w/h: i32/u32,  // pre-zoom geometry
}

struct ShellTab {
    surface_id: u64,
    title_id: u64,
    flags: u32,
}
```

- `FRAMES: [Option<ShellFrame>; 9]` -- static frame array
- `WINDOWS: Vec<WindowState>` -- legacy window state (heap-backed)
- `SceneId(u8)`, `FrameId(u32)`, `TabIndex(u8)` -- type-safe new-id wrappers
- `frame_for_surface(sid)` -- lookup frame containing a surface
- `active_surface_for_frame(fid)` -- get active tab's surface

### 1.5 Close/Minimize/Zoom Code (IMPLEMENTED)

| Action | Function | Line |
|--------|----------|------|
| Close | `close_surface_from_frame_light(sid)` | 13809 |
| Closeable check | `is_closeable_surface(sid)` | 13777 |
| Minimize | `minimize_frame(fid)` | 13988 |
| Restore | `restore_minimized_frame(fid)` | 14039 |
| First minimized | `first_minimized_frame_id()` | 13974 |
| Zoom | `zoom_frame(fid)` | 14248 |
| Unzoom | `unzoom_frame(fid)` | 14309 |
| Top bar toggle | `toggle_top_bar_for_active_frame()` | 15478 |

### 1.6 Lifecycle FSM (IMPLEMENTED)

8-state canonical FSM defined in `LifecycleState` enum (line 5097):
`Allocated -> Mapped -> Visible <-> Hidden/Minimized -> Closing -> Tombstoned -> Destroyed`

- `SurfaceLifecycle { state, generation }` -- per-surface metadata
- `LIFECYCLE_TABLE: [Option<(u64, SurfaceLifecycle)>; 32]` -- static table
- `LIFECYCLE_GENERATION: u64` -- monotonic generation counter
- `lifecycle_register()`, `lifecycle_state()`, `set_lifecycle_state()` -- FSM ops
- `FocusRef { surface_id, generation }` -- stale reference detection
- `TombstoneEvent` ring buffer (8 entries) -- debug observability
- `record_tombstone_event()` -- records death transitions

### 1.7 Atlas/Restore State

- Minimize snapshot: frame flags `FRAME_FLAG_MINIMIZED`, lifecycle state `Minimized`
- Zoom snapshot: `normal_x/y/w/h` fields in `ShellFrame`, flag `FRAME_FLAG_ZOOMED`
- Restore: `restore_minimized_frame()` re-activates via 0xEC, clears minimized flag
- Atlas overlay: toggled via F10, surface ID `SURFACE_ID_ATLAS_OVERLAY`

### 1.8 Frame Lights / Chrome Rendering (IMPLEMENTED in sexdisplay)

Located in `servers/sexdisplay/src/main.rs`:
- Neon rim (4px, color `0x00B4BEFE`) on focused frame
- Frame lights: red (close, `0x00FF4444`), yellow (minimize, `0x00FFCC44`), green (zoom, `0x0044FF44`)
- Top bar chrome (28px header band)
- Tab strip rendering (active/inactive tabs)
- Hover-reveal for single-tab frames
- Bounds checks: every pixel write guarded against framebuffer dimensions
- Chrome flags communicated via `OP_SURFACE_TAB_INFO` PDX call from silk-shell

### 1.9 Dead-Surface Cleanup

- `clear_focus_if_dead()` -- z-order fallback
- `clear_drag_if_dead()` -- cancel drag, record tombstone
- `clear_hover_if_dead()` -- clear hover on dead surface
- `clear_drag_if_wrong_scene()` -- cancel drag after scene switch
- Tombstone ring prevents re-focus of recently closed surfaces
- `surface_is_alive()` -- checks ALIVE booleans + low-level liveness
- `surface_is_lifecycle_focusable()` -- Visible/Mapped only

### 1.10 Existing Proof Markers (sample)

| Existing Marker | Semantics |
|-----------------|-----------|
| `[lifecycle.state.init]` | Lifecycle model initialized |
| `[lifecycle.transition.allow]` | FSM transition accepted |
| `[lifecycle.generation.bump]` | Generation incremented |
| `[shell.focus.clear_dead]` | Focus cleared from dead surface |
| `[shell.drag.clear_dead]` | Drag cancelled on dead surface |
| `[shell.hover.clear.dead]` | Hover cleared on dead frame |
| `[tombstone.event.record]` | Tombstone recorded |
| `[frame.light.minimize.fsm]` | Minimize FSM executed |
| `[shell.interact.minimize]` | Minimize action completed |
| `[shell.interact.restore]` | Restore action completed |
| `[silk.frame.lights.render]` | Frame lights drawn in sexdisplay |
| `[silk.frame.rim.render]` | Neon rim drawn |
| `[silk.frame.lights.render.bounds]` | Bounds check passed |
| `[sexdisplay.frame.light.chrome.recv]` | Chrome descriptor received |

---

## 2. RISK TABLE

| Risk | Severity | Mitigation | Status |
|------|----------|------------|--------|
| Static ID collision (no dynamic allocation) | Low | Fixed-range constants, no reuse | Acceptable for V1 |
| `WINDOWS: Vec` heap-backed | Medium | `FRAMES` array is static; `WINDOWS` is legacy | Monitor; A5 migration plan exists |
| 0xEE opcode shared (destroy/minimize) | Medium | Differentiated by lifecycle state post-call | A5/A7 audit deferred |
| Generation wraparound at 0 | Low | Saturates, does not wrap to 0 | Handled |
| No process-kill semantics | Low | Intentional -- out of scope | Won't fix in V1 |
| Hover state stale after minimize | Low | Explicit `clear_hover_if_dead()` call | Mitigated |
| Close disabled globally (close_allowed=0) | Info | Red light rendered dim; close action blocked | Feature flag pending |
| No storage persistence | Info | Intentional -- out of scope | Won't fix in V1 |

---

## 3. PATCH PLAN

### Phase 1 -- Audit (COMPLETE)
- File: `docs/handoff/WINDOW_APP_LIFECYCLE_V1.md` (this document)
- No code changes

### Phase 2 -- Lifecycle Model Markers
- Add `[silk.lifecycle.init]` at lifecycle_init_all() completion
- Add `[silk.lifecycle.surface.live]` at lifecycle_init_all() boot path
  (RELOCATED: originally at surface_is_lifecycle_live() — dead code, zero callers)
- Add `[silk.lifecycle.focus.clear_dead]` at clear_focus_if_dead()
- Add `[silk.lifecycle.restore.record]` at restore_minimized_frame()
- Add `[silk.lifecycle.invariant.ok]` at end of lifecycle_init_all()
- File: `servers/silk-shell/src/main.rs`

### Phase 3 -- Close / Tombstone Markers
- Add `[silk.lifecycle.close.begin]` at close_surface_from_frame_light()
- Add `[silk.lifecycle.tab.close.ok]` on successful tab close
- Add `[silk.lifecycle.frame.empty.destroy]` when last tab removed
- Add `[silk.lifecycle.focus.next_live]` at fallback focus after close
- Add `[silk.lifecycle.dead.reject]` at dead-surface input rejection
- Add `[silk.lifecycle.close.done]` at close completion
- File: `servers/silk-shell/src/main.rs`

### Phase 4 -- Minimize / Restore Markers
- Add `[silk.lifecycle.minimize.begin]` at minimize_frame()
- Add `[silk.lifecycle.minimize.snapshot]` at minimize snapshot capture
- Add `[silk.lifecycle.minimize.hidden]` at surface deactivation
- Add `[silk.lifecycle.restore.begin]` at restore_minimized_frame()
- Add `[silk.lifecycle.restore.ok]` on successful restore
- Add `[silk.lifecycle.restore.reject_dead]` when restore blocked by dead surface
- File: `servers/silk-shell/src/main.rs`

### Phase 5 -- Zoom / Unzoom Markers
- Add `[silk.lifecycle.zoom.begin]` at zoom_frame()
- Add `[silk.lifecycle.zoom.snapshot]` at geometry snapshot save
- Add `[silk.lifecycle.zoom.active]` at zoom layout activation
- Add `[silk.lifecycle.zoom.restore]` at unzoom / geometry restore
- Add `[silk.lifecycle.zoom.reject_dead]` when zoom blocked
- File: `servers/silk-shell/src/main.rs`

### Phase 6 -- Renderer Frame Lights Markers
- Add `[sexdisplay.frame_lights.render.begin]` at render entry
- Add `[sexdisplay.frame_lights.draw.ok]` at successful light draw
- Add `[sexdisplay.frame_lights.bounds.ok]` at bounds check pass
- File: `servers/sexdisplay/src/main.rs`

---

## 4. STOP FIRST RISKS

| Condition | Status | Notes |
|-----------|--------|-------|
| Requires kernel edit | NO | All changes in silk-shell/sexdisplay only |
| Requires sex-pdx ABI edit | NO | Existing opcodes sufficient |
| Requires new syscall | NO | Existing PDX calls used |
| Requires compositor protocol redesign | NO | OP_SURFACE_TAB_INFO already exists |
| Requires shared/backing buffer redesign | NO | No buffer changes needed |
| Requires cross-PD raw pointer design | NO | All via PDX calls |
| Requires sexdisplay lifecycle ownership | NO | sexdisplay is render-only |
| Requires broad frame-tree refactor | NO | Model already exists |
| Requires storage persistence | NO | Out of scope |
| Requires app process kill semantics | NO | Out of scope |
| Removes existing proof markers | NO | Additive only |
| Weakens existing gate | NO | Additive only |
| Touches USB/input driver policy | NO | No input driver changes |
| Changes scheduler/PKRU/time behavior | NO | No kernel changes |

**STOP FIRST TRIGGERS: NONE** -- all phases are additive marker insertions only.

---

## 5. PHASE STATUS

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | Audit current lifecycle reality | DONE |
| 2 | Add lifecycle model markers | DONE |
| 3 | Safe close / tombstone markers | DONE |
| 4 | Minimize / restore markers | DONE |
| 5 | Zoom / unzoom markers | DONE |
| 6 | Renderer frame lights markers | DONE |

---

## 6. FILES TO PATCH

- `servers/silk-shell/src/main.rs` -- Phases 2-5 marker insertions
- `servers/sexdisplay/src/main.rs` -- Phase 6 marker insertions
- `docs/handoff/WINDOW_APP_LIFECYCLE_V1.md` -- this file, updated per phase

## Build Rule Correction

Do not call `make iso` directly for lifecycle/runtime probes.

The ISO build path is sealed to:

```bash
./scripts/entrypoint_build.sh
```

---

## 7. BEHAVIOR CORRECTION (2026-05-20) — WINDOW_LIFECYCLE_BEHAVIOR_PROOF_FIX_V1

### 7.1 Fix 1 — Real tab removal in close_surface_from_frame_light()

**Problem:** `[silk.lifecycle.frame.empty.destroy]` was unreachable. `close_surface_from_frame_light()` set ALIVE=false and lifecycle=Destroyed, but never removed the surface from `ShellFrame.tabs[]` or decremented `tab_count`. The frame-empty check at the end of the function called `frame_tab_count(fid)` which always returned the original tab_count (never 0).

**Fix:** After surface deactivation, the function now:
1. Scans FRAMES for the frame containing surface_id
2. Clears the matching tab slot in `frame.tabs[]`
3. Left-compacts remaining tabs (no holes)
4. Decrements `frame.tab_count`
5. Adjusts `frame.active_tab` if it pointed at or after the removed slot
6. If `frame.tab_count == 0`, emits `[silk.lifecycle.frame.empty.destroy]` and clears the FRAMES slot

**Result:** `[silk.lifecycle.frame.empty.destroy]` is now reachable by code path. Close now actually removes tab membership and destroys empty frames.

### 7.2 Fix 2 — Move sexdisplay Phase 6 markers into real render path

**Problem:** Three Phase 6 proof markers were hardcoded boot announcements in `_start()`:
- `[sexdisplay.frame_lights.render.begin]`
- `[sexdisplay.frame_lights.draw.ok]`
- `[sexdisplay.frame_lights.bounds.ok]`

These fired once at boot with hardcoded values (fb_w=1024, fb_h=768) and did not prove real rendering had occurred.

**Fix:** Removed the three markers from `_start()`. Added them as one-shot budgeted markers in `composite_pixel()`:
- `render.begin` — emits at first entry to focused-surface pixel path (after clamp_surface bounds check)
- `bounds.ok` — emits at same location, proving bounds were validated by clamp_surface + sw/sh guards
- `draw.ok` — emits alongside existing `FRAME_LIGHT_STARTUP_RENDER_BUDGET` in the frame light rendering block

**Result:** Phase 6 markers now prove real rendering path execution. Confirmed in QEMU runtime output with actual framebuffer dimensions (fb_w=1280, fb_h=800).

### 7.3 Markers preserved

All existing markers remain in place:
- `[silk.lifecycle.close.begin]` — unchanged
- `[silk.lifecycle.tab.close.ok]` — unchanged
- `[silk.lifecycle.frame.empty.destroy]` — now reachable (was dead code)
- `[silk.lifecycle.focus.next_live]` — unchanged
- `[silk.lifecycle.dead.reject]` — unchanged
- `[silk.lifecycle.close.done]` — unchanged

### 7.4 Verification

- **Build:** `./scripts/entrypoint_build.sh` — PASS
- **Runtime probe:** `/tmp/window_lifecycle_runtime_probe.sh` — PASS (no faults)
- **Phase 6 markers in render path:** CONFIRMED (QEMU serial output shows dynamic fb_w/fb_h)
- **frame.empty.destroy reachable:** YES (code path exists; not triggered during 45s smoke window)
- No kernel edits, no ABI changes, no protocol redesign
