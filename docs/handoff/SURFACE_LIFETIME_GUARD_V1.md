# SURFACE_LIFETIME_GUARD_V1

## Status

Implemented (2026-05-04). One self-defending guard added to `point_in_surface()`.
All existing lifetime safety verified.

---

## Surface Lifetime Model

### Surface IDs (hardcoded constants)

| ID | Constant | Alive source | Can die? |
|----|----------|-------------|----------|
| 100 | `SURFACE_ID_APP` | `SURFACE_100_ALIVE` | Yes (DestroyFocused) |
| 101 | `SURFACE_ID_STATIC` | `SURFACE_101_ALIVE` | Yes |
| 102 | `SURFACE_ID_TEST3` | `SURFACE_102_ALIVE` | Yes |
| 103 | `SURFACE_ID_TEST4` | `SURFACE_103_ALIVE` | Yes |
| 200 | `SURFACE_ID_LINEN` | Always true | No (static) |
| 0x90 | `SURFACE_ID_CURSOR` | Always true | No (OS-owned) |
| 0x92 | `SURFACE_ID_LAUNCHER` | `LAUNCHER_ACTIVE` | Yes (toggle open/close) |
| 0x93 | `SURFACE_ID_STATUS` | `STATUS_ACTIVE` | Yes |
| 0x94 | `SURFACE_ID_CLOCK` | `CLOCK_ACTIVE` | Yes |
| 0x95 | `SURFACE_ID_BELL` | `BELL_ACTIVE` | Yes |

### Death representation

Death is represented as a boolean `false` in a per-surface static variable.
No surface memory is freed; position data persists after death.
Recreation resets the alive flag and re-initializes position/size.

### Validity helper

```rust
fn surface_is_alive(sid: u64) -> bool { ... }
```

Already existed and was used in:
- `clear_focus_if_dead()` — clears focus if target dead
- `clear_drag_if_dead()` — clears drag if target dead
- `try_set_focus()` — rejects dead surfaces
- `click_hit_test_and_focus()` — skips dead surfaces in z-order iteration
- `drag_move_focused()` — checks `SURFACE_NNN_ALIVE` per surface

---

## Audit Findings

### Pre-patch gap

**`point_in_surface()` did not check alive status.** It relied on callers having called
`clear_focus_if_dead()` first. This precondition-based pattern meant any new call site
that forgot the precondition could hit-test a dead surface and select it as a click target.

### Affected paths

| Path | Precondition guard | After patch |
|------|--------------------|-------------|
| Focused surface check (line 492) | Caller called `clear_focus_if_dead()` | Self-defending via `surface_is_alive()` |
| Z-order iteration (line 501) | Already had `surface_is_alive()` guard | Redundant but harmless |
| Drag-start check (line 520) | Implicit via caller | Self-defending |
| Budget markers (line 760) | No guard | Self-defending (more accurate) |

### Other paths verified safe

| Path | Guard | Status |
|------|-------|--------|
| Focus write (`try_set_focus`) | `surface_is_alive()` | ✅ |
| Focus dead-clearing (`clear_focus_if_dead`) | `surface_is_alive()` | ✅ |
| Drag dead-clearing (`clear_drag_if_dead`) | `surface_is_alive()` | ✅ |
| Hit-test z-order (`click_hit_test_and_focus`) | `surface_is_alive()` per surface | ✅ |
| Drag movement (`drag_move_focused`) | `SURFACE_NNN_ALIVE` per branch | ✅ |
| Snapshot emit (`emit_snapshot`) | `SURFACE_NNN_ALIVE` per surface | ✅ |
| Cursor surface update | `SURFACE_ID_CURSOR` always alive | ✅ |
| Arrow key movement | `SURFACE_NNN_ALIVE` per branch | ✅ |

---

## Patch

### `servers/silk-shell/src/main.rs`

One guard added to `point_in_surface()` (line 273):

```rust
if !surface_is_alive(sid) {
    serial_println!("[shell.surface.dead.skip] id={} reason=inactive", sid);
    return false;
}
```

This makes `point_in_surface` **self-defending** — it never returns `true` for a dead/inactive surface regardless of caller precondition. The existing precondition guards remain for clarity and early bail-out.

### Markers

- New: `[shell.surface.dead.skip] id=N reason=inactive` — fires when `point_in_surface` is called on an inactive panel or dead app surface.
- All existing markers preserved unchanged.

---

## Build

```bash
./scripts/entrypoint_build.sh
```

Both default and synthetic (`SEXUSB_SYNTHETIC=1 SEXOS_PROOFS_DISABLED=1`) pass.

---

## Verification

```bash
SEXUSB_QEMU_DEVICE=mouse SEXOS_QEMU_DISPLAY=sdl-grab ./dev.sh run 2>/dev/null | tee /tmp/surface-lifetime-guard-v1.log

for m in \
  shell.drag.start \
  shell.drag.move \
  shell.drag.end \
  shell.click_focus \
  shell.surface.dead.skip \
  shell.cursor_surface.move.ok \
  sexdisplay.cursor.surface.update
do
  printf "%-42s %d\n" "$m" "$(grep -ac "\[$m\]" /tmp/surface-lifetime-guard-v1.log)"
done
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/surface-lifetime-guard-v1.log
```

Pass criteria:
- `shell.drag.start/move/end` > 0 (drag lifecycle intact)
- faults = 0

---

## Remaining Risks

- **No surface registry**: Surface IDs are hardcoded constants. No allocator, no ID reuse tracking. Future Frame Chrome/tabs will need a proper registry with allocate/free/lookup.
- **Position data persists after death**: A dead surface's position/size fields remain in memory. `point_in_surface()` now returns false for them, but any code that reads position data directly (bypassing `point_in_surface()`) could see stale values. All such code paths currently check `SURFACE_NNN_ALIVE` explicitly.
- **`surface_is_alive()` uses panel active booleans**: For panels, "alive" = active/open. This is correct because an inactive panel has no visible surface. The marker changed from `nonfocusable.reject` to `dead.skip` for inactive panels, which is more accurate.
- **All-dead focus window**: If all four app surfaces are destroyed, `FOCUSED_SURFACE_ID` still points to the last destroyed surface until `clear_focus_if_dead()` runs. `point_in_surface()` now returns false for it, so hit-test won't select it. Focus remains stale but harmless.

---

## Next Recommended Phase

**EVENT_ORDERING_CONTRACT_V1** — deterministic event processing order in `silk-shell`:
1. Receive bounded input events
2. Normalize/update pointer state
3. Hit-test (now self-defending against dead surfaces)
4. Update interaction state
5. Apply shell command/focus decision
6. Emit display/model updates
7. Yield

This is the last subcontract before `INTEGRATED_SCENARIO_PROOF_V1` — a multi-phase synthetic proof exercising focus, drag, hit-test, and surface destruction in sequence.
