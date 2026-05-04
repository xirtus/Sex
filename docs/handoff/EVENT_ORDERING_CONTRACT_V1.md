# EVENT_ORDERING_CONTRACT_V1

## Status

Implemented (2026-05-04). Audit complete — no patch needed. Canonical event processing order defined and verified. All existing ordering invariants are correct.

---

## Canonical Event Processing Order

When any input event arrives at silk-shell, processing follows this deterministic order:

| Step | Phase | What happens | Paths |
|------|-------|-------------|-------|
| 1 | Receive | `pdx_listen_raw(0)` — block until event arrives | All ingress |
| 2 | Classify | `msg.type_id` dispatch: `OP_USB_MOUSE_REPORT` or `OP_HID_EVENT` (sub-dispatch `EV_KEY`/`EV_ABS`/`EV_REL`/`EV_BTN`) | All ingress |
| 3 | Preflight | `clear_focus_if_dead()` + `clear_drag_if_dead()` — surface-lifetime guards run before any focus/drag operation | USB (720-721), EV_BTN (1521-1522) |
| 4 | Normalize | Update `POINTER_BUTTONS`, `POINTER_WHEEL_ACCUM`, or `POINTER_X`/`POINTER_Y` from event payload | USB (733-734), EV_ABS (1448-1449), EV_REL (1472-1473), EV_BTN (1513-1516) |
| 5 | Key action | `EV_KEY` dispatch: scancode → SurfaceAction (FocusToggle, DestroyFocused, arrow keys, snap, resize, etc.) | EV_KEY (824-1444) |
| 6 | Hit-test | Left-button down edge → `try_transition(ClickPending)` → `click_hit_test_and_focus(px, py, buttons)` — SilkBar, focused surface, z-order fallback, focus switch, drag start | USB (755-758), EV_BTN (1526-1529) |
| 7 | Release | Button-up edge → ClickPending→Idle (cancel) or Dragging→Idle (drop) | USB (780-791), EV_BTN (1550-1561) |
| 8 | Movement | `clear_drag_if_dead()` → `drag_move_focused(dx, dy)` — move drag target surface by delta | USB (810-813), EV_REL (1480-1483) |
| 9 | Update cursor | `pdx_call(SLOT_DISPLAY, OP_SURFACE_UPDATE, SURFACE_ID_CURSOR, POINTER_X, POINTER_Y)` — emit cursor position to compositor | USB (793-795), EV_REL (1497-1499) |
| 10 | Emit snapshot | `if mutated { emit_snapshot(); }` — push window descriptors to compositor | After any mutation |
| 11 | Yield | `sys_yield()` — relinquish timeslice | Always |

### Step 10 Detail: emit_snapshot()

When `mutated == true`, `emit_snapshot()` (line 220) performs:

1. Collect up to 16 window descriptors into `SNAPSHOT[0..15]`
2. Sort by focus priority (focused window last = topmost in z-order)
3. `pdx_call(SLOT_DISPLAY, OP_DISPLAY_SET_SNAPSHOT, ...)` — push snapshot
4. Per-surface position updates via `OP_SURFACE_UPDATE`:
   - Surface 100 (if alive): from `WINDOWS[1].desc.x/y`
   - Surface 101 (if alive): from `SURFACE_101_X/Y`
   - Surface 102 (if alive): from `SURFACE_102_X/Y`
   - Surface 103 (if alive): from `SURFACE_103_X/Y`

Note: Cursor surface update (Step 9) happens unconditionally and independently of the `mutated` flag.

---

## Ingress Path Table

| Event type | Entry | Steps executed | Comments |
|-----------|-------|---------------|----------|
| `OP_USB_MOUSE_REPORT` | line 703 | 1→3→4→6→7→8→9→10→11 | Synthetic proof path. dx/dy NOT applied to POINTER_X/Y (see line 729). |
| `OP_HID_EVENT` / `EV_KEY` | line 824 | 1→2→5→10→11 | Keyboard actions only. `mutated` set by action handlers. Does NOT update pointer state. |
| `OP_HID_EVENT` / `EV_ABS` | line 1447 | 1→2→4→10→11 | Absolute position update. No interaction logic (no button state change). |
| `OP_HID_EVENT` / `EV_REL` | line 1451 | 1→2→4→8→9→10→11 | Relative mouse movement. Drag movement + cursor update. |
| `OP_HID_EVENT` / `EV_BTN` | line 1509 | 1→2→3→4→6→7→10→11 | Real-button path. Hit-test, drag start/end. |
| `OP_SHELL_BIND_BUFFER` | line 675 | 1→10→11 | Init only. Sets `mutated = true`. |

### EV_KEY + EV_REL + EV_BTN in a single OP_HID_EVENT

If sexinput packs multiple event classes into one `OP_HID_EVENT`, the dispatch order within the `unsafe` block is:

```
EV_KEY  →  arrow keys  →  EV_ABS  →  EV_REL  →  EV_BTN
```

This means:
- Keyboard actions (FocusToggle, DestroyFocused) are processed **before** pointer-movement or button events from the same batch. This is correct — keyboard-initiated focus changes during drag use `InteractionState::Dragging.surface_id`, not `FOCUSED_SURFACE_ID`, so a FocusToggle cannot corrupt the drag target.
- EV_REL movement is processed **before** EV_BTN button-up edge. If drag-move arrives in the same event as button-release, the movement applies first (harmless — drag target already clamped), then Dragging→Idle transition fires.
- EV_BTN always runs after pointer state (EV_ABS/EV_REL) is already updated, ensuring hit-test sees the latest pointer position.

---

## Audit Findings

### Pre-patch gap

**None.** The event processing order is correct across all paths:

| Check | Result |
|-------|--------|
| Dead surfaces cleared before interaction? | ✅ USB (720-721), EV_BTN (1521-1522). Both paths call `clear_focus_if_dead()` + `clear_drag_if_dead()` before hit-test or movement. |
| Pointer state normalized before hit-test? | ✅ POINTER_BUTTONS set before hit-test in USB (733) and EV_BTN (1513-1516). POINTER_X/Y set before hit-test via EV_ABS (1448-1449) or EV_REL (1472-1473). |
| Hit-test runs before drag start? | ✅ `click_hit_test_and_focus()` runs before `try_transition(Dragging)` — in fact, drag start is the last thing click_hit_test_and_focus() does (line 526-531). |
| Drag movement uses Dragging::surface_id? | ✅ Already proven in SHELL_INTERACTION_STATE_V1. `drag_move_focused()` reads from `InteractionState::Dragging`, not `FOCUSED_SURFACE_ID`. |
| Cursor surface updated after pointer state? | ✅ USB (793-795), EV_REL (1497-1499). Cursor update always reads latest POINTER_X/Y. |
| Snapshot emitted after all mutations? | ✅ Single `if mutated { emit_snapshot() }` at line 1571-1573, after all branches. |
| All-dead focus window handled? | ✅ `clear_focus_if_dead()` in preflight handles stale focus. `point_in_surface()` self-defends via SURFACE_LIFETIME_GUARD_V1. |
| No event type changes pointer state in inconsistent order? | ✅ Each event class independently manages its own state fields. No cross-class data races (single-threaded dispatch). |

### Invariants preserved

1. **Single-threaded dispatch**: `pdx_listen_raw(0)` blocks for the next event. No preemption between steps 3-9 within a dispatch.
2. **Preflight before interaction**: `clear_focus_if_dead()` and `clear_drag_if_dead()` are called before any code that reads focus or drag state for decision-making.
3. **State normalized before decision**: POINTER_X/Y and POINTER_BUTTONS are set before hit-test, drag-move, or focus-change logic reads them.
4. **Cursor update is unconditional**: Every USB and EV_REL event updates the cursor surface position via PDX call, regardless of `mutated` flag.
5. **Snapshot after mutation**: The `mutated` flag tracks whether any focus/surface/position state changed. `emit_snapshot()` runs iff `mutated == true`.
6. **Yield at end of loop**: `sys_yield()` always runs at line 1575, ensuring the compositor (sexdisplay) gets a timeslice.

---

## Patch

**No code changes needed.** All event-processing order invariants are already correctly enforced. The only deliverable is this handoff document.

Rationale:
- `clear_focus_if_dead()` and `clear_drag_if_dead()` are correctly placed in both USB and EV_BTN paths
- Pointer state normalization happens before hit-test and movement in all paths
- Hit-test and drag-start are correctly ordered within `click_hit_test_and_focus()`
- Snapshot emission is correctly gated on `mutated` after all branches
- No duplicate, misplaced, or missing operations

---

## Build

```bash
./scripts/entrypoint_build.sh
```

Both default and synthetic (`SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1`) pass.

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/event-ordering-contract-v1.log

for m in \
  shell.focus.set \
  shell.focus.clear \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end \
  shell.click_focus \
  shell.cursor_surface.move.ok \
  sexdisplay.cursor.surface.update
do
  printf "%-42s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/event-ordering-contract-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/event-ordering-contract-v1.log
```

Pass criteria:
- `shell.drag.start` > 0, `shell.drag.move` > 0, `shell.drag.end` > 0
- `shell.focus.set` > 0
- faults = 0

Detailed step-order proof (one event at a time):

```bash
# Extract the first click-focus event sequence from a synthetic run
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 \
  SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab \
  ./dev.sh run 2>/dev/null | head -500 | grep -E \
  "(shell\.recv|shell\.pointer|shell\.focus|shell\.click_focus|shell\.drag|shell\.cursor_surface)" \
  > /tmp/event-order-1.log
```

Expected order for a click-drag-release sequence:
```
[shell.recv.usb_mouse]
[shell.pointer.usb_state.start]
[shell.pointer.usb_state.ok]              ← step 4: normalize
[shell.click_focus.down]                  ← step 6: hit-test
[shell.click_focus.hit]                   ←   (inside click_hit_test_and_focus)
[shell.focus.set]                         ←   (inside try_set_focus)
[shell.drag.start]                        ←   (drag start)
[shell.click.real.target]                 ←   (budget marker)
[shell.cursor_surface.move.start]         ← step 9: cursor update
[shell.cursor_surface.move.ok]
[shell.drag.move]                         ← step 8: drag movement (later frame)
[shell.drag.send.ok]
[shell.cursor_surface.move.start]         ← step 9: cursor update (same frame)
[shell.cursor_surface.move.ok]
[shell.drag.end]                          ← step 7: release
[shell.cursor_surface.move.start]         ← step 9: cursor update (release frame)
[shell.cursor_surface.move.ok]
```

---

## Remaining Risks

- **No formal cross-class event ordering**: If sexinput packs EV_KEY + EV_REL + EV_BTN into a single OP_HID_EVENT, the dispatch order within the unsafe block (EV_KEY → arrow → EV_ABS → EV_REL → EV_BTN) is an emergent property of the code structure, not a documented contract. Future refactors could accidentally reorder these blocks. A compile-time ordering assertion or a single dispatch function with explicit step comments would prevent regression.
- **Cursor update before snapshot in EV_REL path**: In EV_REL (lines 1481-1499), drag movement sets `mutated = true`, then cursor update fires, then `emit_snapshot()` runs at line 1572. The snapshot includes the moved surface position but the cursor has already been updated. This is fine — cursor is independent of the snapshot — but worth noting for future compositor work where cursor z-order might interact with surface z-order.
- **`mutated` not set by cursor update**: Cursor surface position changes do not trigger `emit_snapshot()`. If sexdisplay ever needs snapshot data to position the cursor compositing layer, this would need to change.
- **USB path does both movement and cursor update**: The OP_USB_MOUSE_REPORT path (synthetic proof) performs `drag_move_focused()` AND cursor surface update in the same dispatch. Synthetic proof steps include dragging with the USB absolute path (dx/dy from the synthetic report), but `POINTER_X/Y` is NOT updated from those dx/dy values (documented at line 729). This means cursor position stays at the last HID EV_ABS position, while the drag target surface moves. This is intentional and correct for the synthetic proof.

---

## Next Recommended Phase

**INTEGRATED_SCENARIO_PROOF_V1** — multi-phase synthetic proof exercising focus, drag, hit-test, and surface destruction in sequence, verifying all previously hardened subcontracts work together:

1. Surface 100 created and focused (boot state, already proven)
2. Click-to-focus on surface 101 via EV_ABS + EV_BTN
3. Drag surface 101 right/down
4. Press FocusToggle (keyboard focus change during drag — verifies drag target stability)
5. Continue dragging (verifies drag continues on original target, not new focal surface)
6. Release and verify final position
7. DestroyFocused (keyboard action)
8. Verify focus auto-switches to next alive surface
9. RecreateFocused and verify surface re-initializes

This phase closes out the silk-shell subcontract series and proves the complete interaction pipeline.
