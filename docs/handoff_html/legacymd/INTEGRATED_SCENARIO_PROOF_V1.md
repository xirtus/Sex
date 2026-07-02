# INTEGRATED_SCENARIO_PROOF_V1

## Status

Implemented (2026-05-04). One integrated 242-frame synthetic proof exercising the hardened shell interaction contracts together via the sexusb USB mouse path.

---

## Scenario Proven

The synthetic proof (`SEXUSB_SYNTHETIC=1`) executes a deterministic 242-frame drag sequence that validates all hardened shell contracts in a single integrated run:

| Phase | Frames | Action | Contract(s) exercised |
|-------|--------|--------|----------------------|
| 1 | 40 | Cursor drifts right/down (dx=1, dy=1, buttons=0) toward surface 100 | EVENT_ORDERING (receive→normalize→cursor update), SURFACE_LIFETIME (clear_focus_if_dead runs each frame) |
| 2 | 1 | Button down (buttons=1, dx=0, dy=0) → ClickPending → click_hit_test_and_focus → focus set → drag start | HIT_TEST_PRIORITY (focused surface check), FOCUS_CONTRACT (try_set_focus via click_hit_test_and_focus), SHELL_INTERACTION_STATE (ClickPending→Dragging transition), SURFACE_LIFETIME (point_in_surface self-defends) |
| 3 | 80 | Drag right with button held (buttons=1, dx=1, dy=0) | SHELL_INTERACTION_STATE (drag_move_focused reads Drag::surface_id), SURFACE_LIFETIME (clear_drag_if_dead runs before each move) |
| 4 | 80 | Drag down with button held (buttons=1, dx=0, dy=1) | Same as Phase 3 |
| 5 | 1 | Button release (buttons=0, dx=0, dy=0) → Dragging→Idle | SHELL_INTERACTION_STATE (clean transition) |
| 6 | 40 | Cursor drifts back (dx=-1, dy=-1, buttons=0) | EVENT_ORDERING (post-drag state stable) |

### Integrative diagnostics

- `[integrated.proof.start]` — proof boot marker
- `[integrated.proof.phase] n=N label` — phase boundary markers (N=1..6)
- `[shell.integrated.drag_target] id=N focus=N` — logs drag target surface_id and current FOCUSED_SURFACE_ID on each drag move (budgeted, max 4 emissions). When `id == focus`, drag target matches focused surface (normal case). Proves `drag_move_focused()` reads from `InteractionState::Dragging`, not `FOCUSED_SURFACE_ID`.

---

## Scenario Not Proven and Why

### FocusToggle during drag (Goal item 4)

**Not covered.** Silk-shell handles `EV_KEY` → `FocusToggle` (scancode 0x0F) via `OP_HID_EVENT` from sexinput, but:
- sexusb synthetic path only sends `OP_USB_MOUSE_REPORT` (mouse reports, not keyboard)
- sexinput `OP_USB_KEYBOARD_REPORT` handler (line 242) only maps USB HID usage IDs to EV_REL cursor movement for keyboard cursor fallback — it does NOT forward arbitrary scancodes to silk-shell as EV_KEY events
- sexinput is forbidden to modify per INTEGRATED_SCENARIO_PROOF_V1 scope
- Silk-shell has no existing synthetic keyboard/focus trigger hook
- Adding keyboard synthesis (sexusb→OP_USB_KEYBOARD_REPORT→sexinput→EV_KEY→silk-shell) would require nontrivial protocol changes across the pipeline

**Impact:** The `[shell.integrated.drag_target]` marker logs `id=N focus=N` but in the current proof both values are always equal (no FocusToggle fires during drag). The code path is verified correct by inspection (`drag_move_focused()` unconditionally reads `InteractionState::Dragging.surface_id`, not `FOCUSED_SURFACE_ID`, as proven in SHELL_INTERACTION_STATE_V1).

### Dead/inactive surface hit-test skip (Goal item 6)

**Not covered.** Surface destruction (DestroyFocused) requires keyboard scancode 0x3C, which cannot be synthesized through the mouse-only sexusb path. All panels (launcher, clock, status, bell) are toggled via SilkBar clicks, which require SilkBar interaction during the synthetic proof. No existing safe synthetic surface-destruction mechanism exists.

**Impact:** `point_in_surface()` self-defends against dead surfaces as proven in SURFACE_LIFETIME_GUARD_V1. The guard is verified correct by audit and code inspection.

---

## Changes

### servers/sexusb/src/main.rs

Added 9 `serial_println!` calls for integrated proof markers (no frame count change, no behavioral change):

```rust
serial_println!("[integrated.proof.start]");
// Before each phase:
serial_println!("[integrated.proof.phase] n=N label");
// After Phase 6:
serial_println!("[integrated.proof.complete]");
```

Existing markers: `[sexusb.synthetic.start]`, `[sexusb.synthetic.drag.start]`, `[sexusb.synthetic.drag.frame]`, `[sexusb.synthetic.drag.complete]`, `[sexusb.synthetic.complete.ok]` — all preserved.

### servers/silk-shell/src/main.rs

Added budgeted diagnostic marker inside `drag_move_focused()` at line 479:

```rust
unsafe {
    static mut INTEGRATED_DRAG_TARGET_BUDGET: u32 = 4;
    let b = &mut INTEGRATED_DRAG_TARGET_BUDGET;
    if *b > 0 {
        *b -= 1;
        serial_println!("[shell.integrated.drag_target] id={} focus={}", surface_id, FOCUSED_SURFACE_ID);
    }
}
```

---

## Build

```bash
# Default (no synthetic)
./scripts/entrypoint_build.sh

# Synthetic proof
SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1 ./scripts/entrypoint_build.sh
```

Both pass cleanly (no warnings in changed files).

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/integrated-scenario-proof-v1.log

for m in \
  integrated.proof.start \
  integrated.proof.phase \
  integrated.proof.complete \
  sexinput.usb_mouse.recv \
  sexinput.usb_mouse.normalize.ok \
  shell.click_focus.hit \
  shell.focus.set \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end \
  shell.cursor_surface.move.ok \
  sexdisplay.cursor.surface.update \
  shell.surface.dead.skip \
  shell.integrated.drag_target
do
  printf "%-42s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/integrated-scenario-proof-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/integrated-scenario-proof-v1.log
```

### Expected counts

| Marker | Expected | Proves |
|--------|----------|--------|
| `integrated.proof.start` | 1 | Proof booted |
| `integrated.proof.phase` | 6 | All 6 phases executed |
| `integrated.proof.complete` | 1 | Proof completed |
| `sexinput.usb_mouse.recv` | 242 | All 242 frames received |
| `sexinput.usb_mouse.normalize.ok` | 242 | All 242 frames normalized |
| `shell.click_focus.down` | ≥1 | Hit-test ran on click |
| `shell.focus.set` | ≥1 | Focus applied to surface |
| `shell.drag.start` | 1 | Drag started |
| `shell.drag.move` | ≥1 | Drag moved (160 frames × 2 paths = up to 320, but only from EV_REL path = 160) |
| `shell.drag.end` | 1 | Drag ended |
| `shell.cursor_surface.move.ok` | ≥1 | Cursor updates sent to display |
| `sexdisplay.cursor.surface.update` | ≥1 | Display received cursor updates |
| `shell.surface.dead.skip` | 0 | No dead surface hit-test attempted (expected — no surfaces die during proof) |
| `shell.integrated.drag_target` | ≤4 | Budgeted drag-target diagnostic fired |
| faults | 0 | Memory safety |

### Pass criteria

- `integrated.proof.start` == 1
- `integrated.proof.complete` == 1
- `shell.drag.start` > 0, `shell.drag.move` > 0, `shell.drag.end` > 0
- `shell.focus.set` > 0
- faults == 0

---

## Remaining Risks

- **FocusToggle-during-drag not empirically proven**: The `[shell.integrated.drag_target]` marker always shows `id == focus` in the current proof because no FocusToggle fires during drag. The independence of `Drag::surface_id` from `FOCUSED_SURFACE_ID` is verified by code audit only (SHELL_INTERACTION_STATE_V1). A future keyboard-synthesis phase could add `OP_USB_KEYBOARD_REPORT` forwarding through sexinput to trigger FocusToggle mid-drag and prove `id != focus` empirically.
- **Dead-surface skip not empirically proven**: `point_in_surface()` self-defends against dead surfaces as proven in SURFACE_LIFETIME_GUARD_V1, but no synthetic test exercises the `[shell.surface.dead.skip]` code path. A future phase with keyboard-synthesized DestroyFocused would exercise this path.
- **Single surface only**: The proof drags surface 100 (APP) only. Switching to another app surface via click hit-test is not demonstrated because surface 100's large boot size (800×500) covers all other surfaces; the cursor never exits surface 100's bounds during the 242-frame sequence. A future proof could position the cursor into a different surface area by using a longer drift phase or EV_ABS anchoring.
- **No faults ≠ no UB**: The fault scan detects page faults and general protection faults at the x86-64 level. Undefined behavior that does not manifest as a hardware fault (e.g., stale pointer read, integer overflow in position calculation) is not detected by this proof. The clamping logic (`clamp_position`) bounds surface positions, and all position operations use `wrapping_add` which is defined behavior in Rust.

---

## Next Recommended Phase

Two possible continuations depending on priority:

### FRAME_CHROME_MODEL_V1
Design a surface ID registry and frame/tab chrome layer for the shell:
1. Replace hardcoded surface ID constants with a registry (allocate/free/lookup)
2. Add window frame chrome (title bar, close/minimize/maximize buttons)
3. Add tab bar chrome for multi-surface windows
4. Define frame chrome hit-test priority (above app surfaces, below SilkBar)

### SELECTED_WINDOW_SILKBAR_OPTIONS_V1
Add the missing Frame Chrome model to SilkBar:
1. Surface ID registry for dynamic surface creation/destruction
2. Window chrome with title bar, close button, minimize, maximize
3. Chrome mode arbitration between app surfaces and chrome elements
4. Hit-test priority update to include chrome layers

Recommended: **FRAME_CHROME_MODEL_V1** — the surface ID registry is a prerequisite for dynamic window creation.
