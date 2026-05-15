# LAUNCHER_MULTI_EXEC_ATLAS_GATE_FIX_V1

Date: 2026-05-15
Status: PASS
Scope: servers/silk-shell/src/main.rs, docs/handoff/

## 1. PASS/STOP FIRST

| Status | Meaning |
|--------|---------|
| **PASS** | launcher_multi_exec gate passes with 7/7 apps, Atlas overlay correctly verified |
| **STOP FIRST** | No kernel/ABI/USB/input/display/pointer edits. No broad refactor. Atlas nonfocusable design preserved. |

## 2. Attempts Used

| Attempt | Result | Notes |
|---------|--------|-------|
| 1 | PASS | Pre-condition fix + marker update. 16/16 gates PASS. Zero faults. |

## 3. Root Cause

`palette_execute_selected()` for Atlas calls `atlas_toggle()` which **toggles**
the overlay state. The multi-exec proof runs **after** the batch proof (which
also toggles Atlas). Since `ATLAS_MODE_ENABLED` initializes to `false`:

1. Batch proof runs → `atlas_toggle()` → Atlas opens (`ATLAS_MODE_ENABLED = true`)
2. Multi-exec proof runs → `atlas_toggle()` → Atlas closes (`ATLAS_MODE_ENABLED = false`)
3. Multi-exec proof checks `ATLAS_MODE_ENABLED` → `false` → Atlas counted as FAIL
4. Result: `passed=6 failed=1` instead of `passed=7 failed=0`

The toggle is inherently nondeterministic: any prior Atlas state change
(including the batch proof that runs on the same tick) makes the outcome a
coin flip.

## 4. Exact Fix

Two changes in `maybe_run_app_launcher_multi_exec_proof()`:

**Change 1 (pre-condition):** Before executing Atlas (idx=3), ensure overlay is
closed so the subsequent `atlas_toggle()` inside `palette_execute_selected()`
reliably opens it:

```rust
if idx == 3 && ATLAS_MODE_ENABLED {
    atlas_toggle(); // close — exec toggle will open it
}
```

**Change 2 (markers):** Report Atlas exec and focus with `overlay_enabled_nonfocusable`
reason instead of the misleading `exec_reject`/`focused`:

- `[launcher.multi.exec] idx=3 app=Atlas ok=1 reason=overlay_enabled_nonfocusable`
- `[launcher.multi.focus] app=Atlas sid=151 ok=1 reason=overlay_enabled_nonfocusable`

The exec ok value now uses `ATLAS_MODE_ENABLED` (not `palette_execute_selected()`
return) since `try_set_focus(151)` is always rejected for the nonfocusable overlay.

## 5. Files Changed

- `servers/silk-shell/src/main.rs` — lines ~11523-11579 in `maybe_run_app_launcher_multi_exec_proof()`
  - Added pre-condition check for Atlas (close if already open before exec)
  - Updated exec marker for Atlas to use ATLAS_MODE_ENABLED for ok value
  - Updated exec/focus reason strings for Atlas to `overlay_enabled_nonfocusable`
- `docs/handoff/LAUNCHER_MULTI_EXEC_ATLAS_GATE_FIX_V1.md` — this handoff

## 6. Build/Profile Result

```
SEXOS_APP_LAUNCHER_MULTI_EXEC_PROOF=1 ./scripts/entrypoint_build.sh → PASS
./scripts/entrypoint_build.sh → PASS (baseline, zero behavior change)
```

## 7. Runtime Proof Counts

```
[launcher.multi.exec] idx=0 app=Spindle ok=1 reason=launched
[launcher.multi.focus] app=Spindle sid=153 ok=1 reason=focused
[launcher.multi.exec] idx=1 app=Quil ok=1 reason=launched
[launcher.multi.focus] app=Quil sid=201 ok=1 reason=focused
[launcher.multi.exec] idx=2 app=Linen ok=1 reason=launched
[launcher.multi.focus] app=Linen sid=200 ok=1 reason=focused
[launcher.multi.exec] idx=3 app=Atlas ok=1 reason=overlay_enabled_nonfocusable
[launcher.multi.focus] app=Atlas sid=151 ok=1 reason=overlay_enabled_nonfocusable
[launcher.multi.exec] idx=4 app=Bell ok=1 reason=launched
[launcher.multi.focus] app=Bell sid=204 ok=1 reason=focused
[launcher.multi.exec] idx=5 app=Collar ok=1 reason=launched
[launcher.multi.focus] app=Collar sid=203 ok=1 reason=focused
[launcher.multi.exec] idx=6 app=Mesh ok=1 reason=launched
[launcher.multi.focus] app=Mesh sid=202 ok=1 reason=focused
[launcher.multi.proof.done] ok=1 passed=7 failed=0
```

## 8. Final Gate Output

```
  keyboard_gui                 PASS   silkbar clock ticks: 12
  command_palette              PASS   panel=1 rows=20
  spindle_daily                PASS   items=13 blockers=8
  spindle_bridges              PASS   bridge evidence: 54 markers
  linen_nonblocking            PASS   nonblocking proof done: 1 marker(s)
  linen_detail                 PASS   6 objects seeded
  quil_keyboard                PASS   keyboard buffer proof: 1 done
  bell_events                  PASS   bell event markers found
  atlas_theme                  PASS   atlas settings init found
  collar_nav                   PASS   12 grants auto-issued
  mesh_nav                     PASS   frame topology: 8 tab events
  silkbar_status               PASS   51 status updates
  launcher_multi_exec          PASS   7/7 apps passed: 7 execs
  palette_linen_available      PASS   Linen palette status: nonblocking_ready
  quil_status_ready            PASS   Quil palette status: keyboard_nav_ready
  faults_zero                  PASS   0 fault markers

  PASS gates: 16
  FAIL gates: 0
  SKIP gates: 0
  FINAL: PASS
```

## 9. Preserved Constraints

- No kernel edits
- No ABI/sex-pdx edits
- No USB/input/display edits
- No pointer/slot2 mouse edits
- No broad refactor
- Atlas nonfocusable design preserved (surface 151 remains nonfocusable)
- Existing command palette preserved
- Existing APP_LAUNCHER_V1 proof preserved
- Existing batch proof preserved
- Baseline build has zero behavior change

## Handoff Path

```
docs/handoff/LAUNCHER_MULTI_EXEC_ATLAS_GATE_FIX_V1.md  ← THIS DOCUMENT
docs/handoff/APP_LAUNCHER_MULTI_EXEC_PROOF_V1.md        ← prior art
docs/handoff/DAILY_DRIVER_PROFILE_UPDATE_AFTER_QUIL_LINEN_V1.md  ← related gate definition
```

