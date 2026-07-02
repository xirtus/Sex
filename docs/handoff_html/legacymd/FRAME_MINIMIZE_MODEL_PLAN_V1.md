# FRAME_MINIMIZE_MODEL_PLAN_V1

## Status

Design (2026-05-04). Analysis of minimize/collapse feasibility. No code changed.

---

## Current Surface Active/Inactive/Destroy Semantics

### Shell side (silk-shell)

| State | Mechanism | Meaning |
|-------|-----------|---------|
| Alive | `SURFACE_*_ALIVE = true` | Surface exists, can be focused, hit-testable |
| Destroyed | `SURFACE_*_ALIVE = false` + `0xEE` | Surface gone, focus/drag guard, no restore |

Shell has no intermediate "hidden" or "minimized" state. A surface is either alive or dead.

### Display side (sexdisplay)

```
0xEE (surface destroy):
  slot.active = false              ← just a flag
  if FOCUSED_SURFACE_ID == target:
    FOCUSED_SURFACE_ID = 0         ← display-side focus clear
  redraw_surface_area(...)          ← re-render without this surface

0xEC (surface create/upsert):
  Phase 1: find ACTIVE slot with matching surface_id → update geometry
  Phase 2: if not found, find first INACTIVE slot → fill with new Surface { active: true, ... }
```

**Key insight:** `0xEE` does NOT free the slot or clear geometry data. It only sets `active = false`. The geometry (`x, y, w, h`), color, and fill rect fields remain in the slot. They're simply skipped by `composite_pixel()`.

---

## Proposed Minimize Model

### Shell-side: Frame flag for minimized state

ShellFrame already has a reserved `flags: u32` field (line 222) — can use bit 0 for MINIMIZED.

```rust
const FRAME_FLAG_MINIMIZED: u32 = 1 << 0;
```

No new fields needed — the existing `flags` field was designed for exactly this purpose.

### Display-side: Reuse 0xEE/0xEC (no new IPC needed)

| Operation | IPC | Display effect |
|-----------|-----|---------------|
| **Minimize** (hide) | `0xEE(surface_id)` | `slot.active = false` → surface invisible |
| **Restore** (show) | `0xEC(surface_id, x, y, w, h)` | New slot with `active = true` → surface visible |

**Why this works:**
- `0xEE` simply sets `active = false`. The slot remains allocated with its geometry data.
- `0xEC` upsert: Phase 1 finds no active slot with matching ID (because inactive), Phase 2 finds an inactive slot and creates a fresh entry with `active: true`.
- Shell stores geometry in static muts (`SURFACE_100_X/Y/W/H`, etc.) — can pass these to `0xEC` on restore.

### Maximize surface count safety

```
MAX_SURFACES = 16
Current active surfaces (V1): ~10
  APP(100), STATIC(101), TEST3(102), TEST4(103),
  LINEN(200), CURSOR(0x90), LAUNCHER(0x92),
  STATUS(0x93), CLOCK(0x94), BELL(0x95)

Spare slots: 6

Each minimize frees 1 slot (sets active=false).
Each restore consumes 1 new slot (old slot remains inactive, orphaned).
```

**Slot leak:** Each minimize→restore cycle leaves one inactive orphan slot. With 6 spare slots and 4 closeable surfaces (100-103), worst case is 4 cycles before pressure. Acceptable for V1 but should be documented.

---

## Restore Mechanism (V1)

Without a tab bar or taskbar, the user needs a way to restore a minimized frame.

### Options considered

| Option | Complexity | UX | Recommended for V1? |
|--------|-----------|-----|-------------------|
| **Keyboard shortcut** | Low — add `SurfaceAction::RestoreMinimized` | Press key to cycle through minimized frames | **✅ Yes** |
| MINIMIZE light toggle | Impossible — surface is hidden, lights invisible | Can't click what isn't rendered | ❌ |
| SilkBar indicator | Medium — add minimized chip/icon | Good UX but needs SilkBar extension | Future |
| Auto-restore on focus | Low — `try_set_focus` re-activates minimized | Surprising behavior | ❌ |

### V1 recommendation: Keyboard shortcut

Add a `SurfaceAction::RestoreMinimized` (or `SurfaceAction::ToggleMinimize`) bound to a key. When pressed:
1. Find the first minimized frame
2. Restore it (send 0xEC with stored geometry)
3. Focus it via `try_set_focus()`

---

## Focus/Drag Invariants

| Invariant | How it's maintained |
|-----------|-------------------|
| Focus does not remain on minimized surface | `clear_focus_if_dead()` called after minimize — `surface_is_alive()` still returns true (alive flag unchanged), BUT we add a check for minimized flag. Alternative: set `SURFACE_*_ALIVE = false` on minimize and re-enable on restore. |
| Drag clears if target minimized | `clear_drag_if_dead()` checks `surface_is_alive()`. If minimize sets alive=false, drag clears automatically. |
| No display draws minimized surface | `0xEE` sets `slot.active = false` — `composite_pixel()` skips inactive surfaces |

### Design decision: Minimize should NOT set SURFACE_*_ALIVE = false

Using the `minimized` flag in ShellFrame instead of repurposing the alive flag:
- The alive flag means "surface can never be shown again" (one-way)
- The minimized flag means "surface is temporarily hidden" (reversible)
- `surface_is_alive()` returns true for minimized surfaces (but they're not interactive)
- Hit-test (`point_in_surface`) should skip minimized surfaces via a new check
- Focus/drag guards need to check minimized flag in addition to alive

**Alternative (simpler):** Set `SURFACE_*_ALIVE = false` on minimize, `SURFACE_*_ALIVE = true` on restore. This reuses all existing alive/dead guards. But it conflates "destroyed" with "minimized" — focus fallback messages would say "dead" instead of "minimized."

**Recommended:** Use a separate minimized flag for clarity. Add a `surface_is_interactive()` helper that checks both alive and minimized:

```rust
unsafe fn surface_is_interactive(sid: u64) -> bool {
    surface_is_alive(sid) && !frame_containing_surface_is_minimized(sid)
}
```

---

## Verdict

### MINIMIZE_MODEL_SAFE_NOW ✅

| Requirement | Feasible? | How |
|-------------|-----------|-----|
| Must not destroy surface | ✅ | `0xEE` only sets `active = false` — slot data preserved |
| Restore possible | ✅ | `0xEC` upsert re-activates with stored geometry |
| Focus not remain on hidden | ✅ | `clear_focus_if_dead()` or explicit focus fallback |
| Drag clears on hidden | ✅ | `clear_drag_if_dead()` or explicit drag-end |
| sexdisplay sole writer | ✅ | Shell triggers 0xEE/0xEC, sexdisplay renders |
| No kernel ABI change | ✅ | 0xEE/0xEC already exist |
| No broad compositor rewrite | ✅ | Only ShellFrame.flags bit + click handler changes |

---

## Implementation Plan

### Files to modify

| File | Changes |
|------|---------|
| `servers/silk-shell/src/main.rs` | Add `FRAME_FLAG_MINIMIZED`, minimize/restore helpers, click handler branch for MINIMIZE light, keyboard action |

### Files NOT modified

- `kernel/` — no ABI changes
- `crates/sex-pdx/` — no protocol changes
- `crates/silkbar-model/` — no model changes
- `servers/sexdisplay/` — no renderer changes (reuses 0xEE/0xEC as-is)
- `servers/silkbar/` — no forwarding changes
- `servers/sexusb/` — no synthetic proof changes
- `servers/sexinput/` — untouched

### Exact changes

#### 1. Add frame flag constant

```rust
/// ShellFrame.flags: frame is minimized (hidden, not destroyed).
const FRAME_FLAG_MINIMIZED: u32 = 1 << 0;
```

#### 2. Add minization helpers

```rust
/// Returns true if the frame containing this surface is minimized.
unsafe fn frame_is_minimized_for_surface(surface_id: u64) -> bool {
    frame_for_surface(surface_id).map_or(false, |fid| {
        FRAMES.iter().any(|f| {
            f.map_or(false, |frame| {
                frame.frame_id == fid && (frame.flags & FRAME_FLAG_MINIMIZED) != 0
            })
        })
    })
}

/// Minimize the active surface of the given frame.
/// Hides the surface via 0xEE and flags the frame as minimized.
unsafe fn minimize_frame(frame_id: u32, surface_id: u64) -> bool {
    if !surface_is_alive(surface_id) { return false; }
    // Mark frame as minimized
    for f in FRAMES.iter_mut() {
        if let Some(frame) = f {
            if frame.frame_id == frame_id {
                frame.flags |= FRAME_FLAG_MINIMIZED;
                break;
            }
        }
    }
    // Hide from display
    pdx_call(SLOT_DISPLAY, 0xEE, surface_id, 0, 0);
    // Clear drag if dragging this surface
    clear_drag_if_dead(); // will also check surface_is_alive
    // Fall back focus
    clear_focus_if_dead();
    true
}

/// Restore a minimized frame: re-activate its surface via 0xEC and clear the flag.
unsafe fn restore_minimized_frame(frame_id: u32, surface_id: u64) -> bool {
    // Verify frame is minimized and surface is still alive
    for f in FRAMES.iter_mut() {
        if let Some(frame) = f {
            if frame.frame_id == frame_id {
                frame.flags &= !FRAME_FLAG_MINIMIZED;
                break;
            }
        }
    }
    if !surface_is_alive(surface_id) { return false; }
    // Re-create surface on display (upsert to inactive slot)
    let (x, y, w, h) = get_surface_bounds(surface_id).unwrap_or((100, 100, 800, 500));
    pdx_call(SLOT_DISPLAY, 0xEC, surface_id,
        (y as u64) << 32 | x as u64,
        (h as u64) << 32 | w as u64);
    try_set_focus(surface_id);
    true
}
```

#### 3. Modify `click_hit_test_and_focus()` for MINIMIZE light

In the `else if light != FRAME_LIGHT_NONE` branch (currently captures MINIMIZE/ZOOM as no-op):

```rust
} else if light == FRAME_LIGHT_MINIMIZE {
    // ── MINIMIZE action: hide active surface ──
    if let Some(surface_id) = active_surface_for_frame(frame_id) {
        if surface_is_alive(surface_id) {
            minimize_frame(frame_id, surface_id);
            // budgeted [shell.frame.light.minimize] marker
        }
    }
} else if light == FRAME_LIGHT_ZOOM {
    // ── ZOOM light: no action in V1, capture ──
    // existing capture/no-op
```

#### 4. Add keyboard restore action

```rust
SurfaceAction::RestoreMinimized => {
    // Find first minimized frame and restore it
    for f in FRAMES.iter() {
        if let Some(frame) = f {
            if (frame.flags & FRAME_FLAG_MINIMIZED) != 0 {
                if let Some(tab) = &frame.tabs[frame.active_tab as usize] {
                    restore_minimized_frame(frame.frame_id, tab.surface_id);
                }
                break;
            }
        }
    }
}
```

### Diagnostic markers

| Marker | Budget | Fires |
|--------|--------|-------|
| `[shell.frame.light.minimize] frame=N surface=N` | 8 | MINIMIZE light click hides surface |
| `[shell.frame.light.restore] frame=N surface=N` | 8 | Restore action brings surface back |

### Proof that existing markers still pass

| Marker | Why it still fires |
|--------|-------------------|
| `[shell.frame.rim.drag.start]` | Non-light rim clicks unchanged |
| `[shell.frame.light.close]` | CLOSE light path unchanged |
| `[shell.frame.light.hover]` | Light hover detection unchanged |
| `[shell.selected.options.send]` | Options display unchanged |
| `[shell.drag.move]` | Drag movement unchanged |
| `sexdisplay.selected.options.update` | SilkBar options unchanged |

### Safety edges

| Edge case | Handling |
|-----------|----------|
| Minimize already-minimized frame | `minimize_frame` checks `surface_is_alive()` → still alive, but `FRAME_FLAG_MINIMIZED` already set → no-op (flag already set, 0xEE already sent) |
| Restore non-minimized frame | Helper checks flag → skips |
| Minimize while dragging | `clear_drag_if_dead()` called after minimize → drag target becomes inactive |
| All surfaces minimized | Last surface cannot minimize (or focus goes to 0) |
| Surface destroyed while minimized | `surface_is_alive()` returns false → restore silently fails |

---

## Remaining Risks

- **Slot leak**: Each minimize→restore cycle leaves one inactive orphan slot in sexdisplay. With MAX_SURFACES=16 and ~10 active, limited cycles before pressure. Future phase should add proper slot reuse or a "re-activate" IPC.
- **No visual indicator**: Minimized frames have no visible representation. User can't tell a frame is minimized without a tab bar or taskbar. Keyboard restore requires remembering which frames are minimized.
- **Geometry mismatch on restore**: If the frame was moved while minimized (impossible in V1 — no drag), the restored position might be stale. Current model assumes position doesn't change during minimize.

---

## Next Implementation Phase

### FRAME_MINIMIZE_ACTION_V1

Implementation of MINIMIZE light behavior.

```
MISSION: FRAME_MINIMIZE_ACTION_V1

IMPLEMENTATION ONLY. Design complete in FRAME_MINIMIZE_MODEL_PLAN_V1.md.

Files to modify:
- servers/silk-shell/src/main.rs

Changes:
1. Add FRAME_FLAG_MINIMIZED = 1<<0 constant
2. Add minimize_frame() helper (sets flag, 0xEE, clear_focus_if_dead)
3. Add restore_minimized_frame() helper (clears flag, 0xEC, try_set_focus)
4. Add keyboard SurfaceAction::RestoreMinimized
5. In click_hit_test_and_focus(), MINIMIZE light → minimize_frame()
6. Budgeted [shell.frame.light.minimize] marker

Forbidden:
- Any ABI/opcode change
- Any sexdisplay change
- Any silkbar/silkbar-model change
- Any renderer change
- Close or zoom behavior
- Tab bar implementation
```
