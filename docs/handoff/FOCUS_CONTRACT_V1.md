# FOCUS_CONTRACT_V1

## Status

Implemented (2026-05-04). Focus writes already centralized in `try_set_focus()`. Minor diagnostic markers added. No behavioral changes needed.

---

## Audit Findings

### Focus Ownership

**One source of truth:** `FOCUSED_SURFACE_ID` (`static mut u64`, line 171). All reads and writes are explicit.

**All focus writes are already centralized** through `unsafe fn try_set_focus(sid: u64) -> bool` (line 379). No direct `FOCUSED_SURFACE_ID` writes exist outside this function.

### Guards Inside try_set_focus()

| Guard | Effect | Marker |
|-------|--------|--------|
| `sid == 0` | Clear focus | `[shell.focus.clear]` (new) |
| `!is_focusable_surface(sid)` | Reject OS-owned surfaces (cursor, panels) | `[shell.focus.reject.nonfocusable]` |
| `!surface_is_alive(sid)` | Reject dead surfaces | `[shell.focus.reject.dead]` |
| Success | Apply focus, emit to display via 0xED | `[shell.focus.set]` (new) |

### Focus Transition Reasons (all route through try_set_focus)

| Reason | Trigger | Caller |
|--------|---------|--------|
| Boot default | Static init `= SURFACE_ID_APP` | line 171 (before any code runs) |
| Click app surface | Hit-test in `click_hit_test_and_focus` | line 510 |
| Keyboard FocusToggle | Tab key cycles through surfaces | lines 819-833 |
| Dead surface clearing | `clear_focus_if_dead()` catches stale focus | lines 337, 344 |
| Surface destruction fallback | DestroyFocused handler auto-switches | lines 862-878 |
| Direct FocusN actions | Focus100/101/102/103/200 keys | lines 883-915 |
| RecreateFocused fallback | When all surfaces were dead and recreated | line 960 |

### Secondary Focus State: FOCUS_ID (window_id)

`FOCUS_ID` (line 170) tracks the *window_id* (1 or 2) for snapshot z-ordering. It is separate from `FOCUSED_SURFACE_ID` and has its own legacy toggle (`SurfaceAction::LegacyFocusToggle`). This is a legacy artifact from the boot-time two-window model and does not affect hit-test or drag behavior.

---

## Patch

### `servers/silk-shell/src/main.rs`

Only change: added two `serial_println!` calls inside `try_set_focus()`:

```rust
// On clear (sid == 0):
serial_println!("[shell.focus.clear] id=0");

// On successful set (sid != 0):
serial_println!("[shell.focus.set] id={}", sid);
```

No behavioral changes. No parameter changes. No new guard logic — existing guards were already correct.

### Invariants (pre-existing, verified by audit)

1. **One focus owner**: `FOCUSED_SURFACE_ID` is sole authority.
2. **Focus changes only through one helper**: All writes via `try_set_focus()`.
3. **Click focus uses canonical hit-test**: `click_hit_test_and_focus()` returns the hit surface, then calls `try_set_focus(hit_id)`.
4. **Drag target stable through focus changes**: `drag_move_focused()` reads `InteractionState::Dragging.surface_id`, not `FOCUSED_SURFACE_ID` (from SHELL_INTERACTION_STATE_V1). Focus toggle during drag cannot corrupt drag target.
5. **Dead surface clears safely**: `clear_focus_if_dead()` called before every interaction path. `try_set_focus()` itself rejects dead surfaces.
6. **Shell chrome does not capture focus**: `handle_silkbar_click()` returns `true` without changing focus.
7. **No marker renamed**: All existing proven markers preserved.

### Not Patched (intentionally)

| Item | Reason |
|------|--------|
| `FocusReason` enum | Requires updating ~26 call sites — broad edit. Existing caller markers already provide context. |
| Drag guard in FocusToggle | FocusToggle during drag is harmless — drag target stored in InteractionState. Behavioral change not needed. |
| `FOCUS_ID` / `FOCUS_ID` consolidation | Separate concern (snapshot z-ordering vs interaction focus). Would require architectural change. |
| All-dead focus cleanup | `clear_focus_if_dead()` catches it on next interaction. Edge case not worth patching. |

---

## Build

```bash
./scripts/entrypoint_build.sh
```

Both default and synthetic (`SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1`) pass.

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/focus-contract-v1.log

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
  printf "%-42s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/focus-contract-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/focus-contract-v1.log
```

Pass criteria:
- `shell.focus.set` > 0 (focus was applied to a surface)
- `shell.drag.start/move/end` > 0 (drag lifecycle intact)
- faults = 0

---

## Remaining Risks

- **`FOCUS_ID` vs `FOCUSED_SURFACE_ID` duality**: Two separate focus tracking variables. `FOCUS_ID` is used only for snapshot z-ordering; `FOCUSED_SURFACE_ID` for interaction logic. If they ever diverge, a surface could appear focused (z-order) but not be the interaction target. Currently they're always in sync via the boot init and surface-100 window mapping, but there's no invariant linking them.
- **All-dead focus window**: If all four app surfaces are destroyed, `FOCUSED_SURFACE_ID` still points to the last destroyed surface until `clear_focus_if_dead()` runs on next interaction. Cosmetic only — no crash risk because `try_set_focus()` guards against dead surfaces.
- **`FOCUSED_SURFACE_ID` initialized statically, not through `try_set_focus()`**: Line 171 sets `= SURFACE_ID_APP` at declaration, before any code runs. This cannot go through the helper. The display is also not yet initialized at that point (PDX calls would fail). This is standard and safe for static initialization.

---

## Next Recommended Phase

**EVENT_ORDERING_CONTRACT_V1** — deterministic event processing order in `silk-shell`:
1. Receive bounded input events
2. Normalize/update pointer state
3. Hit-test
4. Update interaction state
5. Apply shell command/focus decision
6. Emit display/model updates
7. Yield

Remaining subcontracts from `SHELL_GLOBAL_INTERACTION_CONTRACT_V1.md` after EVENT_ORDERING_CONTRACT_V1:
- `SURFACE_ID_LIFETIME_V1` — surface ID creation/destruction lifecycle
- `CHROME_MODE_ARBITRATION_V1` — chrome vs app focus arbitration
- `INTEGRATED_SCENARIO_PROOF_V1` — multi-phase scenario proof
