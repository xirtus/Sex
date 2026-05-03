# SURFACE_ID_LIFETIME_PATCH_V1

**Date:** 2026-05-03
**Commit:** `f940f93 fix(shell): guard dead and unknown surface ids`
**Status:** MERGED

## Changes

Shell-only safety guards for surface ID lifetime. No kernel, sex-pdx, sexdisplay, or sexinput changes.

### 1. Completed `surface_is_alive()` for all known IDs

Previously only checked app surfaces (100–103) and linen (200). Now includes:
- `SURFACE_ID_CURSOR` (0x90) → always alive
- `SURFACE_ID_LAUNCHER` (0x92) → via `LAUNCHER_ACTIVE` bool
- `SURFACE_ID_STATUS` (0x93) → via `STATUS_ACTIVE` bool
- `SURFACE_ID_CLOCK` (0x94) → via `CLOCK_ACTIVE` bool
- `SURFACE_ID_BELL` (0x95) → via `BELL_ACTIVE` bool
- Unknown IDs → `[shell.surface.unknown.reject]` marker + `false`

### 2. Added `clear_focus_if_dead()` guard

Called before any focus-dependent operation (click-focus, drag). If `FOCUSED_SURFACE_ID` points to a dead surface:
- Logs `[shell.surface.focus.clear.dead] id=N` (unbudgeted)
- Iterates z-order for first alive surface, sets focus
- Logs `[shell.surface.focus.fallback] id=N` (budgeted, AtomicU32(8))
- If no surfaces alive, clears focus to 0
- Logs `[shell.surface.focus.clear.none]` (unbudgeted)

### 3. Added `clear_drag_if_dead()` guard

Called before drag movement deltas are applied. If `InteractionState::Dragging { surface_id }` points to a dead surface:
- Logs `[shell.surface.drag.cancel.dead] id=N` (unbudgeted)
- Transitions to `InteractionState::Idle`

### 4. Added unknown-ID reject markers

- `[shell.surface.unknown.reject] point_in_surface id=N` — catch-all in `point_in_surface()`
- `[shell.surface.unknown.reject] surface_is_alive id=N` — catch-all in `surface_is_alive()`

### 5. Budgeted accept markers

Hot-path accept/follow markers use `SURFACE_FOCUS_ACCEPT_BUDGET: AtomicU32(8)` following the same pattern as `INTERACTION_LOG_BUDGET`. Reject/dead/error markers are unbudgeted per standing rule.

## Changed Invariants

1. Before any click-focus or drag operation, the focused surface is verified alive. If dead, it is cleared to the nearest alive surface or 0.
2. Before any drag movement, the drag target is verified alive. If dead, the drag is cancelled.
3. `surface_is_alive()` now correctly reports panel surfaces (0x92–0x95) and cursor (0x90).
4. Unknown surface IDs queried via `point_in_surface()` or `surface_is_alive()` produce a reject marker instead of silent failure.

## Marker List

| Marker | Type | Budget | When |
|--------|------|--------|------|
| `[shell.surface.focus.clear.dead]` | error | UNbudgeted | Focused surface found dead |
| `[shell.surface.focus.fallback]` | accept | AtomicU32(8) | Focus moved to fallback surface |
| `[shell.surface.focus.clear.none]` | error | UNbudgeted | No surfaces alive at all |
| `[shell.surface.drag.cancel.dead]` | error | UNbudgeted | Drag target found dead, cancelled |
| `[shell.surface.unknown.reject]` | error | UNbudgeted | Unknown surface ID queried |

## Deferred Items

- Monotonic surface ID allocation — requires ABI change
- Tombstone registry — no ID reuse risk in V1
- Dead PD cleanup — requires kernel PD death notifications
- Linen in `is_shell_surface()` — intentional V1 limitation
- sexdisplay focus validation — safe, no crash risk
- Cascading if-else focus fallback refactor — `clear_focus_if_dead()` adds safety

## Verification

```bash
# Build
./scripts/entrypoint_build.sh

# Boot
timeout 25 qemu-system-x86_64 -M q35 -m 2G -cpu max,+pku \
  -cdrom sexos-v1.0.0.iso -display none \
  -serial file:serial_sid.log -no-reboot

# Verify surface markers (should be 0 in normal boot)
grep -c 'shell.surface' serial_sid.log

# Verify proof markers preserved
grep -c 'shell.interaction.transition' serial_sid.log  # ≤16
grep -c 'shell.interaction.forbidden' serial_sid.log   # 0
grep -cE 'fault|panic' serial_sid.log                   # 0
grep -c 'shell.silkbar.click' serial_sid.log            # ≥7
grep -c 'sexinput.drag_proof' serial_sid.log             # >0
grep -c 'shell.drag.start' serial_sid.log                # >0
grep -c 'shell.launcher.open.ok' serial_sid.log          # ≥1

# Audit gates
./scripts/audit_invariant_gates.sh --working

# Diff proof
git diff --stat
git diff -- kernel crates/sex-pdx servers/sexdisplay servers/sexinput
```

## Verified Results (2026-05-03)

```
transitions: 16
forbidden: 0
faults: 0
surface markers: 0 (no dead surfaces in normal boot)
drag_proof: 1236
silkbar_click: 8
panel opens: 4
panel closes: 2
drag start/end: 94
audit gates: 7/7 PASS
```

## STOP FIRST Conditions (for future patches)

1. kernel/sex-pdx/sexdisplay/sexinput edits
2. surface ID numeric value changes
3. new IPC opcodes or ABI changes
4. monotonic ID allocation without prior approval
5. dead PD cleanup without prior approval
6. static mut + volatile for budget counters — use AtomicU32
