# SELECTED_WINDOW_SILKBAR_OPTIONS_V1

## Status

Implemented (2026-05-04). Model-only: option bit constants, selected-window query helpers, and
budgeted diagnostic markers added to silk-shell. No ABI change, no SilkBar chip extension,
no visual rendering. Action behavior deferred.

---

## SilkBar Data Path Audit

### Current communication flow

```
silk-shell (PDX 6)          silkbar (PDX ?)            sexdisplay (PDX 4)
    │                            │                           │
    │ OP_SILKBAR_WORKSPACE_ACTIVE│                           │
    │ ─────────────────────────→│                           │
    │ OP_SILKBAR_FOCUS_STATE     │                           │
    │   (one-time boot, arg0=1)  │                           │
    │ ─────────────────────────→│                           │
    │                            │ OP_SILKBAR_UPDATE (0xF2) │
    │                            │   SetWorkspaceActive     │
    │                            │   SetWorkspaceUrgent     │
    │                            │   SetChipVisible         │
    │                            │   SetChipKind            │
    │                            │   SetClock               │
    │                            │ ───────────────────────→│
    │                            │                           │
```

### Key findings

| Aspect | Current state |
|--------|--------------|
| SilkBar update model | `SilkBarUpdate` ring buffer, 5 `UpdateKind` discriminants |
| Chip kinds | `ChipKind::{Net, Wifi, Battery, Clock}` — status indicators only |
| Focus tracking from silk-shell | **One-time** `OP_SILKBAR_FOCUS_STATE` with arg0=1 during boot. NOT sent on focus changes. |
| Runtime silk-shell→silkbar | Only `OP_SILKBAR_WORKSPACE_ACTIVE` (workspace switches) |
| silkbar→sexdisplay | `SendUpdate` for workspace/set-chip/clock. Focus state maps to urgent workspace highlight. |

### Why not extend the chip model

Adding selected-window option chips to the SilkBar would require:

1. **New `UpdateKind`** (e.g., `SetSelectedOptions = 6`) — ABI version bump, silkbar-model crate change
2. **New `ChipKind`** or new rendering path — sexdisplay needs new pixel rendering for action chips
3. **New runtime opcode** from silk-shell to silkbar (current `OP_SILKBAR_FOCUS_STATE` is boot-only)
4. **Potential layout change** — option chips need screen space in the top strip

This is a safe, independent phase. Deferred to **SELECTED_WINDOW_OPTIONS_DISPLAY_PLAN_V1**.

---

## Implemented Model

### Constants (silk-shell line 250-257)

```rust
const OPTION_CLOSE: u32 = 1;     // frame can be closed/destroyed
const OPTION_ZOOM: u32 = 2;      // frame can be zoomed/maximized
const OPTION_MINIMIZE: u32 = 4;  // frame can be minimized/hidden
const OPTION_MOVE: u32 = 8;      // frame can be moved via rim drag
```

### Helper functions (silk-shell line 525-547)

| Function | Returns | Logic |
|----------|---------|-------|
| `selected_frame_id()` | `Option<u32>` | `frame_for_surface(FOCUSED_SURFACE_ID)` |
| `selected_surface_id()` | `Option<u64>` | `FOCUSED_SURFACE_ID` if non-zero and alive |
| `selected_window_options_mask()` | `u32` | Bitmask: `OPTION_MOVE` set if frame-owned, others reserved |

### Marker emission (in `try_set_focus()`)

| Path | Marker | Budget |
|------|--------|--------|
| Focus cleared (sid=0) | `[shell.selected.options] frame=0 surface=0 mask=0` | 4 |
| Focus set on surface | `[shell.selected.options] frame=N surface=N mask=0xN` | 8 |

The marker fires on every focus change (surface click, keyboard FocusToggle, DestroyFocused
fallback, auto-switch). This is the correct integration point because `try_set_focus()` is
the single point of truth for all focus writes (proven in FOCUS_CONTRACT_V1).

### V1 mask behavior

| Surface type | In a frame? | Options mask |
|-------------|-------------|-------------|
| APP (100) — frame 1 | ✅ Yes | `OPTION_MOVE` (0x8) |
| STATIC (101) | ❌ No | 0 |
| TEST3 (102) | ❌ No | 0 |
| TEST4 (103) | ❌ No | 0 |
| LINEN (200) | ❌ No | 0 |

---

## Contracts Proven

| Contract | Mechanism | Marker |
|----------|-----------|--------|
| Selected frame resolves from focus | `selected_frame_id()` = `frame_for_surface(FOCUSED_SURFACE_ID)` | `[shell.selected.options] frame=N` |
| Options computed per focus change | Called from `try_set_focus()` on every successful set/clear | `[shell.selected.options]` budgeted |
| MOVE option set for frame-owned | `selected_window_options_mask()` checks frame membership | `mask=0x8` |
| No action behavior | Helpers are query-only, no silkbar/sexdisplay ABI touched | N/A |
| try_set_focus still centralized | No `FOCUSED_SURFACE_ID` writes outside this function | (Pre-existing) |

---

## Changes

### Files modified

`servers/silk-shell/src/main.rs` only:

| Change | Lines | Type |
|--------|-------|------|
| Option bit constants (OPTION_CLOSE/ZOOM/MINIMIZE/MOVE) | 250-257 | Added |
| `selected_frame_id()` helper | 525-527 | Added |
| `selected_surface_id()` helper | 530-534 | Added |
| `selected_window_options_mask()` helper | 537-547 | Added |
| Budgeted marker in try_set_focus (clear path) | 612-620 | Added |
| Budgeted marker in try_set_focus (set path) | 641-652 | Added |

### Files not modified

- `crates/silkbar-model/src/lib.rs` — no ABI change
- `servers/silkbar/src/main.rs` — no new opcodes
- `servers/sexdisplay/src/main.rs` — no renderer change
- `kernel/`, `crates/sex-pdx/` — no kernel/ABI changes

---

## Build

```bash
# Default
./scripts/entrypoint_build.sh

# Synthetic proof
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both pass cleanly. No new warnings.

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/selected-window-silkbar-options-v1.log

for m in \
  shell.selected.options \
  shell.focus.set \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end \
  sexdisplay.cursor.surface.update
do
  printf "%-42s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/selected-window-silkbar-options-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/selected-window-silkbar-options-v1.log
```

### Expected counts (synthetic proof)

| Marker | Expected | Proves |
|--------|----------|--------|
| `shell.selected.options` | ≥1 | Selected options model emits on focus change |
| `shell.focus.set` | ≥1 | Focus was applied (triggers options) |
| `shell.drag.start` | ≥1 | Content drag lifecycle intact |
| `shell.drag.move` | ≥1 | Content drag moves |
| `shell.drag.end` | ≥1 | Content drag ends |
| faults | 0 | Memory safety |

### Pass criteria

- `shell.selected.options` > 0 (model emits)
- `shell.focus.set` > 0 (focus changes still work)
- `shell.drag.start/move/end` > 0 (drag lifecycle intact)
- faults == 0

---

## Remaining Risks

- **Model only, no visual rendering**: Options mask is computed and logged but never displayed.
  Users cannot see close/zoom/minimize/move affordances.
- **No action behavior**: Setting OPTION_CLOSE does not close the surface. Setting OPTION_ZOOM
  does not resize. All bits are query-only in V1.
- **MOVE is always set for frame-owned surfaces**: In V1, surface 100 (frame 1) always has
  MOVE. Future frames may have non-movable surfaces (e.g., dialog boxes pinned to parent).
  The model supports per-frame flags but V1 doesn't use them.
- **CLOSE/ZOOM/MINIMIZE bits never set**: Reserved bits defined but no surface in V1 has
  close/zoom/minimize semantics. Linen (200) is close-able via DestroyFocused but not tracked
  as a frame surface.
- **Non-frame surfaces have 0 mask**: Standalone surfaces 101-103 have no options. They are
  legacy app content that predates the frame model. They remain draggable via content-area drag
  (existing behavior) but are not "selected window" in the frame model sense.

---

## Next Recommended Phase

### SELECTED_WINDOW_OPTIONS_DISPLAY_PLAN_V1

Design how selected-window option chips reach the SilkBar display:

1. Audit whether `OP_SILKBAR_FOCUS_STATE` can be extended from a one-time boot advertisement
   to a live focus-tracking channel (send on every `try_set_focus()`).
2. Design new `UpdateKind::SetSelectedOptions = 6` or a lightweight alternative that conveys
   the options mask to silkbar without full ABI redesign.
3. Determine whether silkbar→sexdisplay `OP_SILKBAR_UPDATE` can carry option chip data, or
   whether a separate display opcode is cleaner.
4. Design option chip visual rendering in sexdisplay (small glyphs for close/zoom/minimize/move).
5. Do not implement action behavior yet.

This is the prerequisite for making selected-window options visible. Without this phase,
the options model exists only in diagnostics.
