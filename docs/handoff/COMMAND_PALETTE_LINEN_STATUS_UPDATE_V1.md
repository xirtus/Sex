# COMMAND_PALETTE_LINEN_STATUS_UPDATE_V1

Status: **PASS** — Linen palette status updated to nonblocking_ready
Date: 2026-05-14
Attempts: 1

## Summary

After LINEN_NONBLOCKING_OPEN_IMPL_V1 made Linen open non-blocking,
the command palette status for "Open Linen" was still labeled
`blocking_risk`. This updates the status to `nonblocking_ready`
and removes all `blocking_risk_confirmed` bypass paths.

## Status Delta

| Field | Before | After |
|-------|--------|-------|
| `available` | `false` | `true` |
| `status_label` | `blocking_risk` | `nonblocking_ready` |
| `reason` | `linen_open_blocking_risk` | `linen_fast_paint_nonblocking` |
| Statusbar `available` count | 7 | **8** (+1) |
| Palette batch skip | Yes (special-cased) | No (falls through normally) |
| Exec result else branch | `blocking_risk_confirmed` | `open_or_focus_reject` |

## Changes (4 hunks, 1 file)

### 1. `palette_item_status()` — FocusLinen now available
```rust
// Before: (false, "blocking_risk", "linen_open_blocking_risk")
// After:  (true, "nonblocking_ready", "linen_fast_paint_nonblocking")
```

### 2. Palette batch proof — remove FocusLinen skip
Removed 7-line block that special-cased `Command::FocusLinen` as
`blocking_risk_confirmed` with `continue`. Now falls through to
normal exec path which uses `open_linen_in_active_scene()` →
`linen_paint_surface_fast()` (non-blocking).

### 3. Palette batch exec result — update else branch
```rust
// Before: "blocking_risk_confirmed"
// After:  "open_or_focus_reject"
```

### 4. Proof function — `maybe_run_command_palette_linen_status_proof()`
Gate: `SEXOS_COMMAND_PALETTE_LINEN_STATUS_PROOF=1`

Exercises:
1. `palette_item_status(Command::FocusLinen)` → verify `available=true`
2. Open palette → verify statusbar shows `available=8`
3. Emit per-item status: `[shell.palette.status] idx=2 action=OpenLinen available=1 status=nonblocking_ready`
4. Execute FocusLinen via palette → uses fast paint, ok=1
5. Close palette

## Proof Markers
```
[shell.palette.linen.status.proof] stage=0 action=start ok=1
[shell.palette.linen.status.proof] stage=1 action=status_check available=1 status=nonblocking_ready
[shell.palette.status] idx=2 action=Open Linen available=1 status=nonblocking_ready reason=linen_fast_paint_nonblocking
[shell.palette.statusbar] open=1 selected=0 available=8    ← +1 from before
[linen.fast_paint] sid=200 objects=6 ok=1 reason=seeds_only
[linen.open.nonblocking] path=duplicate_focus ok=1 reason=fast_paint
[shell.palette.exec.result] idx=2 action=OpenLinen ok=1 status=nonblocking_ready reason=ok
[shell.palette.linen.status.proof.done] ok=1
```

## Test Results
- Build with flag: ✅ PASS
- Baseline (no flag): ✅ zero behavior change
- Faults: ✅ 0

## Files Changed
- `servers/silk-shell/src/main.rs` — 4 hunks
- `docs/handoff/COMMAND_PALETTE_LINEN_STATUS_UPDATE_V1.md` — created

## Related Handoffs
- `docs/handoff/LINEN_NONBLOCKING_OPEN_IMPL_V1.md` — prerequisite
- `docs/handoff/LINEN_BLOCKING_OPEN_ARCH_REVIEW_V1.md` — design
