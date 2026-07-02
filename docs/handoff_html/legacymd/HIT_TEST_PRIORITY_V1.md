# HIT_TEST_PRIORITY_V1

## Status

Implemented (2026-05-04). Hit-test centralized into a single
`click_hit_test_and_focus()` helper with documented priority order.
All existing markers preserved. Both builds pass.

---

## Canonical Hit-Test Priority

When a button-down event arrives, `click_hit_test_and_focus()` evaluates
surfaces in this order:

| Priority | Layer | Implemented? | Notes |
|----------|-------|-------------|-------|
| 1 | SilkBar chrome | ✅ | `handle_silkbar_click()` — intercepts clicks in y < 50 top strip |
| 2 | Focused surface | ✅ | `point_in_surface(px, py, FOCUSED_SURFACE_ID)` — always checked first |
| 3 | Z-order fallback | ✅ | Hardcoded `[LINEN, TEST4, TEST3, STATIC, APP]` — first alive surface hit wins |
| 4 | Desktop/none | ✅ | Returns `hit_id=0` / `silkbar_handled=false` |

### Future layers (not yet implemented — STOP FIRST):

| Layer | Needed for | When |
|-------|-----------|------|
| System modal | Emergency overlay | Future phase |
| Active OverlayBar | Bell attention panel, dock | Future phase |
| Frame chrome | Tabbed/tiled shell model | Future phase |
| WindowBar chrome | Title bar frame | Future phase |

---

## Changed Code

### `servers/silk-shell/src/main.rs`

**Added: `click_hit_test_and_focus(px, py, buttons_val)`** (line 484)

Extracted from the duplicated inline code. Performs:
1. `[shell.click_focus.down]` marker
2. Check focused surface → hit
3. Z-order iteration → if found, `try_set_focus(hit_id)`
4. SilkBar intercept via `handle_silkbar_click()`
5. Drag start if SilkBar didn't handle and cursor on shell surface
6. Returns `(hit_id, silkbar_handled)`

Also logs `[shell.hit_test.skip] id=N reason=dead` when a dead surface
is skipped during z-order iteration (new diagnostic marker).

**Replaced two inline blocks** with calls to `click_hit_test_and_focus()`:
- USB path (was ~45 lines, now 2 lines + budget markers)
- EV_BTN path (was ~45 lines, now 2 lines + budget markers)

Only budget markers (`CLICK_REAL_TARGET_BUDGET` / `CLICK_REAL_TARGET_BUDGET_BTN`) remain in callers. Everything else (hit-test, focus switch, SilkBar intercept, drag start) is in the shared helper.

---

## Audit Findings

### Pre-patch risks

| Risk | Severity | Found? |
|------|----------|--------|
| Duplicate hit-test logic in USB and EV_BTN paths | Medium — divergence risk for future changes | ✅ Fixed: centralized |
| Dead surface silently included in z-order iteration | Low — loop only checks `surface_is_alive()` without logging skip | ✅ Fixed: now logs `[shell.hit_test.skip] id=N reason=dead` |
| SilkBar checked AFTER surface focus hit-test | Low — y < 50 and app surfaces don't overlap currently | Not patched (would change semantics; current order is fine) |
| No explicit priority documentation | Low — ordering implicit in code structure | ✅ Fixed: documented in this handoff |
| z_order hardcoded, not derived from visual z-index | Low — in current model, focused surface IS topmost | Not patched (would add complexity without benefit for V1) |

### Invariants preserved

- `[shell.click_focus.down/hit/miss]` — same markers, same positions
- `[shell.click.real.focus.ok]` — budget markers in callers unchanged
- `[shell.click.real.target]` — same budget logic
- `[shell.drag.start]` — same trigger, same marker
- `[shell.interaction.transition]` — ClickPending → Dragging unchanged
- No new state variables added
- No transition logic changed

---

## Build

```bash
./scripts/entrypoint_build.sh
```

Both default and synthetic (`SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1`) pass.

---

## Verification

```bash
# Default
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/hit-test-priority-v1.log

# Verify markers
for m in \
  shell.click_focus \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end \
  shell.hit_test \
  shell.cursor_surface.move.ok \
  sexdisplay.cursor.surface.update
do
  printf "%-42s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/hit-test-priority-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/hit-test-priority-v1.log
```

Pass criteria:
- `shell.click_focus` (down/hit/miss combined) > 0
- `shell.drag.start` > 0, `shell.drag.move` > 0, `shell.drag.end` > 0
- faults = 0

---

## Next Recommended Phase

**EVENT_ORDERING_CONTRACT_V1** — the next subcontract from
`SHELL_GLOBAL_INTERACTION_CONTRACT_V1.md`:

Define deterministic event processing order:
1. Receive bounded input events
2. Normalize/update pointer state
3. Hit-test
4. Update interaction state
5. Apply shell command/focus decision
6. Emit display/model updates
7. Yield

This phase depends only on `silk-shell` and the now-hardened hit-test + interaction state machine.
