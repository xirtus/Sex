# FRAME_LIGHTS_ACTION_PLAN_V1

## Status

Design (2026-05-04). Analysis of Frame Light click behavior viability. No code changed.

---

## Current Click Path for FrameChrome Hits

```
click_hit_test_and_focus(px, py):
  target = hit_test_at(px, py)
  match target:
    HitTarget::Surface(sid) → focus surface (try_set_focus)
    HitTarget::None → miss, no-op
    HitTarget::FrameChrome { frame_id, kind }:
      if kind == FRAME_CHROME_RIM:
        → rim drag (active_surface_for_frame → try_transition(Dragging))
      else:
        → capture/no-op (tab strip, reserved)
```

**Key observation:** The Frame Light hit detection (`frame_light_at()`) runs in `update_frame_hover_at()`, which updates `HOVERED_FRAME_LIGHT` state. But the click handler does **not** consult `HOVERED_FRAME_LIGHT` — it only sees `HitTarget::FrameChrome { kind: FRAME_CHROME_RIM }`. Frame Light clicks currently fall through to rim drag behavior.

To route light clicks to actions, the click handler needs to check `HOVERED_FRAME_LIGHT` when the target is FrameChrome rim, and branch accordingly.

---

## Current Surface Lifetime Model

| Mechanism | What it does | Used by |
|-----------|-------------|---------|
| `SURFACE_*_ALIVE: bool` | Per-surface alive flag | `surface_is_alive()`, `clear_focus_if_dead()`, `clear_drag_if_dead()` |
| `0xEE` opcode | Tells sexdisplay to mark surface inactive | `toggle_os_panel()`, `SurfaceAction::DestroyFocused` |
| `surface_is_alive()` | Checks alive flag, self-defends | `point_in_surface()`, `try_set_focus()`, frame chrome hit-test |
| `clear_focus_if_dead()` | Auto-moves focus when focused surface dies | Called before focus-dependent operations |
| `clear_drag_if_dead()` | Ends drag when dragged surface dies | Called per event loop |
| `SurfaceAction::DestroyFocused` | Keyboard-triggered destroy (sets ALIVE=false, calls 0xEE, auto-focus fallback) | FocusToggle → D key |

### Surface alive flags are the authoritative dead/alive state

```
surfaces: 100(APP)  101(STATIC)  102(TEST3)  103(TEST4)
alive:    true       true        true        true
          ↓          ↓           ↓           ↓
          SURFACE_100_ALIVE etc. are static mut bool
```

When `SURFACE_*_ALIVE = false`:
- `surface_is_alive()` returns false
- `point_in_surface()` returns false (self-defending)
- `try_set_focus()` rejects dead surfaces
- sexdisplay marks the surface inactive (via 0xEE)
- Frame model (ShellFrame.tabs[]) is NOT updated — stale tab entries remain but are guarded by `surface_is_alive()`

---

## Current Geometry Authority

Shell owns all geometry via static mut positions:
```
SURFACE_100_W/H = 800/500  (WINDOWS[1].desc.x/y for x/y)
SURFACE_101_W/H = 500/300
SURFACE_102_W/H = 350/150
SURFACE_103_W/H = 150/300
```

Sexdisplay receives positions via:
- `0xEC` create (initial x, y, w, h)
- `OP_SURFACE_UPDATE` update (new x, y only — no resize opcode exists)

**No resize mechanism exists.** Surfaces have fixed dimensions. Only position can be changed (via drag).

---

## Risk Table

| Action | Infrastructure Exists? | Safety Concerns | Verdict |
|--------|----------------------|-----------------|---------|
| **CLOSE** (destroy active surface) | ✅ SURFACE_*_ALIVE, 0xEE, focus fallback, drag guard all exist | Must not close linen (desktop). Must guard against close of last surface. Frameless surfaces (linen, cursor) must be rejected. | **CLOSE_SAFE_NOW** |
| **MINIMIZE** (hide/collapse frame) | ❌ No minimize flag, no hide model, no restore path | Needs: minimize flag, hide IPC (or destroy+recreate cycle), tab bar with restore button, z-order management | **BLOCKED** — needs frame state model and IPC extension |
| **ZOOM** (maximize frame) | ❌ No resize mechanism, no save/restore geometry | Needs: resize IPC (or create+destroy cycle), normal/maximized geometry storage, clamp against screen edges | **BLOCKED** — needs resize IPC and geometry state |

---

## Verdict

### CLOSE_SAFE_NOW ✅

The CLOSE light can safely trigger focused surface destruction using the existing infrastructure. The same path as `SurfaceAction::DestroyFocused` (FocusToggle → D key).

### MINIMIZE_BLOCKED ❌

Would require:
1. Frame flag for minimized state
2. Hide/reveal IPC to sexdisplay
3. Tab bar or restore mechanism
4. Z-order reordering on hide/reveal

### ZOOM_BLOCKED ❌

Would require:
1. Resize IPC (new opcode or extend 0xEC)
2. Normal/maximized geometry storage per surface
3. Workspace-aware maximized bounds (avoiding SilkBar)

---

## Exact First Safe Action: CLOSE Light

### Required invariants

1. **No focus dangling**: After close, `try_set_focus(0)` or fallback to next alive surface via existing auto-focus logic
2. **No drag target dangling**: If drag target is being closed, `clear_drag_if_dead()` handles transition to Idle
3. **No display drawing inactive surface**: `0xEE` marks surface inactive in sexdisplay; rim/lights disappear automatically
4. **No kernel/ABI changes**: Zero new opcodes needed — 0xEE already exists
5. **No framebuffer writer violation**: sexdisplay remains sole writer
6. **Must not close desktop (linen)**: `SURFACE_ID_LINEN` has no `SURFACE_200_ALIVE` flag — it's always alive. Guard against closing non-closeable surfaces.
7. **Must not close when no frame is focused**: Only surfaces owned by a frame should be closeable. Linen, cursor, panels are not frame-owned.

### Implementation plan

#### File: `servers/silk-shell/src/main.rs`

**Changes needed:**

1. **In `click_hit_test_and_focus()`**, after the FrameChrome rim drag check (line 1006), add a Frame Light check:

```rust
HitTarget::FrameChrome { frame_id, kind } => {
    if kind == FRAME_CHROME_RIM {
        // Check if pointer is over a Frame Light
        let light = frame_light_at(frame_id, px, py);
        if light == FRAME_LIGHT_CLOSE {
            // Close action: destroy active surface for this frame.
            if let Some(surface_id) = active_surface_for_frame(frame_id) {
                if surface_is_alive(surface_id) && is_closeable_surface(surface_id) {
                    destroy_surface(surface_id);  // set ALIVE=false, 0xEE, focus fallback
                    // budgeted marker
                } else {
                    // reject marker
                }
            }
        } else {
            // Existing rim drag logic unchanged
            ...
        }
    } else {
        // existing capture/no-op
    }
}
```

2. **Add `is_closeable_surface()` guard** — rejects linen, cursor, panels, already-dead surfaces:

```rust
fn is_closeable_surface(sid: u64) -> bool {
    match sid {
        SURFACE_ID_LINEN | SURFACE_ID_CURSOR
        | SURFACE_ID_LAUNCHER | SURFACE_ID_STATUS
        | SURFACE_ID_CLOCK | SURFACE_ID_BELL => false,
        _ => true,
    }
}
```

3. **Reuse existing destroy path** — the same code as `SurfaceAction::DestroyFocused` (lines 1372-1414): set `SURFACE_*_ALIVE = false`, call `pdx_call(SLOT_DISPLAY, 0xEE, surface_id, 0, 0)`, then auto-focus fallback.

4. **Add budgeted diagnostic markers**:
   - `[shell.frame.light.close] frame=N surface=N` — max 8, when CLOSE light click triggers destruction
   - `[shell.frame.light.close.reject] frame=N surface=N reason=...` — unbudgeted, when close rejected

### Files to modify

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | Add `is_closeable_surface()`, modify `click_hit_test_and_focus()` FrameChrome rim arm to check light, add destroy+fallback |

### Files NOT modified

- `kernel/` — no ABI changes
- `crates/sex-pdx/` — no protocol changes
- `crates/silkbar-model/` — no model changes
- `servers/sexdisplay/` — no renderer changes
- `servers/silkbar/` — no forwarding changes
- `servers/sexusb/` — no synthetic proof changes
- `servers/sexinput/` — untouched

### Proof markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.frame.light.close] frame=N surface=N` | 8 | CLOSE light click destroys surface |
| `[shell.frame.light.close.reject] frame=N surface=N reason=dead/invalid` | unbudgeted | CLOSE light click rejected |
| `[shell.frame.light.hover] frame=N light=N` | 8 | Light hover detection (pre-existing) |

### Existing markers that prove no regression

- `[shell.drag.start/move/end]` — rim drag still works for non-light rim clicks
- `[shell.selected.options.send]` — options display intact
- `shell.frame.hover.set` — hover tracking intact
- Fault count = 0

---

## Build & Verification

```bash
# Build both
./scripts/entrypoint_build.sh
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh

# Run
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/frame-lights-action-close-v1.log

# Check markers
for m in \
  shell.frame.light.close \
  shell.frame.light.close.reject \
  shell.frame.light.hover \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end \
  shell.selected.options.send
do
  printf "%-46s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/frame-lights-action-close-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/frame-lights-action-close-v1.log
```

### Pass criteria

- Default build passes
- Synthetic build passes
- `shell.frame.light.close` > 0 when CLOSE light is clicked
- `shell.drag.start/move/end` > 0 (non-light rim drag still works)
- No close action on non-rim clicks, non-closeable surfaces
- No close action on MINIMIZE or ZOOM light clicks
- Focus moves to next alive surface after close
- Faults = 0

---

## Deferred Actions

### MINIMIZE — blocked pending:

1. Frame state model (minimized flag in ShellFrame or per-tab)
2. Hide/reveal IPC (0xEE destroy + create cycle works but loses position — need 0xEB-style update with active flag)
3. Tab bar or restore area (workspace-level shelf for minimized frames)
4. Z-order management (minimized frames go to bottom of z-order)

### ZOOM — blocked pending:

1. Resize IPC (need to send new w/h to sexdisplay — 0xEC create+destroy cycle is destructive)
2. Normal/maximized geometry storage per frame
3. Maximized bounds calculation (screen dimensions minus SilkBar)
4. Unzoom restores original position and size

---

## Next Recommended Phase

### FRAME_LIGHTS_ACTION_CLOSE_V1

Implementation of CLOSE light behavior in silk-shell.

```
MISSION: FRAME_LIGHTS_ACTION_CLOSE_V1

IMPLEMENTATION ONLY. Design complete in FRAME_LIGHTS_ACTION_PLAN_V1.md.

Files to modify:
- servers/silk-shell/src/main.rs

Changes:
1. Add is_closeable_surface() guard function
2. In click_hit_test_and_focus(), FrameChrome rim arm,
   check frame_light_at() before rim drag
3. If light == FRAME_LIGHT_CLOSE: destroy active surface
   (set ALIVE=false, 0xEE, auto-focus fallback)
4. Add budgeted [shell.frame.light.close] marker
5. Budgeted [shell.frame.light.close.reject] for rejected attempts
6. MINIMIZE and ZOOM lights still fall through to rim drag
   (no action behavior yet)

Forbidden:
- Any ABI/opcode change
- Any sexdisplay change
- MINIMIZE/ZOOM action behavior
- Tab management
- Frame geometry changes
- Minimize/zoom IPC
- Surface struct changes

Pass criteria:
- Default build passes
- Synthetic build passes
- CLOSE light destroys focused frame-owned surface
- Non-closeable surfaces (linen, cursor, panels) rejected
- Rim drag still works on non-light rim clicks
- MINIMIZE/ ZOOM lights still fall through to rim drag
- Focus falls back to next alive surface
- No new faults
```
