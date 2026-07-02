# PANEL_TOGGLE_CONSOLIDATION_V1

## Status: PASS (2026-05-03)

## Summary
Consolidated launcher/status/clock panel toggle logic into a single `toggle_os_panel()` helper,
eliminating 42 lines of duplicated open/close surface create/destroy code.

## Changes

### servers/silk-shell/src/main.rs

**Added `toggle_os_panel()` helper** (before `handle_silkbar_click`):
- Takes `active: &mut bool`, `surface_id: u64`, `label: &str`, `x, y, w, h: u32`
- Emits `[shell.{label}.{open,close}.{start,ok}] id={:#x}` markers
- Handles open (0xEC surface create) and close (0xEE surface destroy)

**Consolidated three action handlers**:
- `Action::OpenLauncher` → `toggle_os_panel(&mut LAUNCHER_ACTIVE, SURFACE_ID_LAUNCHER, "launcher", 80, 55, 240, 360)`
- `Action::OpenClock` → `toggle_os_panel(&mut CLOCK_ACTIVE, SURFACE_ID_CLOCK, "clock", 1000, 55, 240, 300)`
- `Action::ToggleModule(_)` → `toggle_os_panel(&mut STATUS_ACTIVE, SURFACE_ID_STATUS, "status", 860, 55, 200, 300)`

**Surface ID registry comment** added near constants:
```
//   0x90  cursor
//   0x92  launcher panel
//   0x93  status/quick-settings panel
//   0x94  clock panel
//   0x95  reserved (Bell panel)
//   100+  app surfaces
```

**Minor marker normalization**: launcher `.open.start` and `.close.start` now include `id=0x92`
suffix for consistency with status/clock panels. Previously they lacked the `id=` suffix.

## Verification
- All three panels open and close correctly
- Workspace switching preserved
- Drag proof preserved
- Contract and render proof pass
- No PF/GP/panic

## Diff Reduction
- Before: ~70 lines of duplicated panel toggle logic
- After: ~30 lines (helper + 3 one-liner call sites)
- ~40 lines saved, zero behavior change
