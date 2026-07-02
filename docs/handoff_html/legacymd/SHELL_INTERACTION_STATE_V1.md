# SHELL_INTERACTION_STATE_V1

## Status

Implemented (2026-05-04). Interaction state unified: drag movement extracted into a single helper that reads the `surface_id` from `InteractionState::Dragging` instead of `FOCUSED_SURFACE_ID`.

---

## Current Interaction State Model

```rust
enum InteractionState {
    Idle,
    ClickPending,
    Dragging { surface_id: u64, current_x: i32, current_y: i32 },
    PanelActive { panel: PanelKind },
}
```

Stored as a single `static mut INTERACTION` (line 199). All transitions go through `try_transition()` which validates the state machine and logs `[shell.interaction.transition]` or `[shell.interaction.forbidden]`.

### Valid transitions

| From | To | Trigger |
|------|----|---------|
| Idle | any | Click, panel toggle |
| ClickPending | Dragging | Left-click on shell surface, cursor within bounds |
| ClickPending | Idle | Button released before drag starts |
| ClickPending | PanelActive | Click on SilkBar chrome |
| Dragging | Idle | Button release |
| PanelActive | Idle | Panel close |
| PanelActive | ClickPending | Click while panel open |

### Forbidden transitions (logged but ignored)

- Dragging → PanelActive
- Dragging → ClickPending
- PanelActive → Dragging

---

## Audit Findings (before patch)

| Risk | Severity | Fixed? |
|------|----------|--------|
| Drag movement reads `FOCUSED_SURFACE_ID` instead of Dragging state's `surface_id` | Medium — keyboard FocusToggle during drag would move wrong surface | ✅ Fixed |
| 34 lines of duplicate drag-move code in USB path and EV_REL path | Low — maintainability | ✅ Fixed |
| `POINTER_BUTTONS` set from two paths (USB absolute, EV_BTN incremental) | Low — sequential, not racy | Not patched (safe by design) |
| `current_x/current_y` stored in Dragging state but never read | Low — informational only | Not patched (harmless) |
| `POINTER_WHEEL_ACCUM` written but never acted on | Low — unused accumulator | Not patched (future use) |

---

## Patch

### `servers/silk-shell/src/main.rs`

**Added: `drag_move_focused(dx: i32, dy: i32) -> bool`**

Shared helper placed after `try_transition()` (line 438). Reads the drag target `surface_id` from `InteractionState::Dragging` (not `FOCUSED_SURFACE_ID`), applies delta, clamps, logs `[shell.drag.move]` and `[shell.drag.send.ok]`, returns `true` if moved.

**Replaced two inline drag-move blocks** with single-line calls to `drag_move_focused()`:
- USB path (was ~34 lines, now 3 lines)
- EV_REL path (was ~34 lines, now 3 lines)

Net change: -24 lines. All existing markers preserved.

### No other files changed

---

## Invariants

1. **One interaction state source**: `INTERACTION` is the sole authority (`static mut`, no duplicates).
2. **No stale drag target**: Drag movement always reads `surface_id` from the Dragging state, not from `FOCUSED_SURFACE_ID`. A keyboard focus change during drag cannot corrupt the target.
3. **Dead target clears safely**: `clear_drag_if_dead()` is called before every drag movement and before every click-to-focus path. Dead surfaces transition Dragging → Idle.
4. **Button down records target**: Hit-test runs before drag start; the clicked surface becomes the drag target (via `FOCUSED_SURFACE_ID` which is stored into the Dragging state).
5. **Release ends cleanly**: Both USB and EV_BTN paths transition Dragging → Idle on button release.
6. **Markers preserved**: `[shell.drag.start]`, `[shell.drag.move]`, `[shell.drag.send.ok]`, `[shell.drag.end]`, `[shell.interaction.transition]` all unchanged.

---

## Build

```bash
# Default
./scripts/entrypoint_build.sh

# Synthetic drag proof (optional verification)
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both builds pass cleanly.

---

## Verification (optional QEMU run)

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/shell-interaction-state-v1.log

for m in \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end \
  shell.cursor_surface.move.ok \
  sexdisplay.cursor.surface.update
do
  printf "%-42s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/shell-interaction-state-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/shell-interaction-state-v1.log
```

Pass criteria:
- `shell.drag.start` > 0
- `shell.drag.move` > 0
- `shell.drag.end` > 0
- faults = 0

---

## Remaining Risks

- **`current_x/current_y` in Dragging state is stored but never used for movement** — only `surface_id` is extracted by `drag_move_focused()`. The position fields are informational (logged at drag.start/end). No behavioral impact.
- **`POINTER_BUTTONS` dual-path update** (USB absolute + EV_BTN incremental) — not a race since both paths are sequential within the same PDX dispatch, but adds a subtle dependency: the EV_BTN path must see the same initial state the USB path set.
- **`POINTER_WHEEL_ACCUM` written but never consumed** — dead accumulator, harmless.

---

## Next Recommended Phase

**HIT_TEST_PRIORITY_V1** — the next subcontract from `SHELL_GLOBAL_INTERACTION_CONTRACT_V1.md`:

Define strict z-order and input capture hierarchy in `silk-shell`:
1. emergency/system modal
2. active OverlayBar
3. armed Bell action surface
4. SilkBar chrome
5. DockBar/EdgeBar chrome
6. WindowBar chrome
7. app surfaces (topmost first)
8. desktop/background

This phase depends only on `silk-shell` and the now-proven interaction state machine. No kernel/display/input changes needed.
