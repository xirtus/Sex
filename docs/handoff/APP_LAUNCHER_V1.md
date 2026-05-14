# APP_LAUNCHER_V1

## Handoff Date
2026-05-14

## Status
PASS

## Contract
- servers/silk-shell/src/main.rs
- docs/handoff/APP_LAUNCHER_V1.md

## Summary
Added app-launcher markers and a proof function to the existing command palette
in silk-shell.  The command palette already acts as an app launcher (7 app rows
for Spindle/Quil/Linen/Atlas/Bell/Collar/Mesh plus 3 window commands).  This
change adds explicit `[launcher.*]` serial markers that call out the app
launcher subset and a proof function that exercises launcher open/nav/exec/close.

No new UI, no duplicate code — markers and proof only.  All existing palette
behavior is preserved.

## Launcher App Table

| idx | App     | Status              | Available |
|-----|---------|---------------------|-----------|
| 0   | Spindle | ready               | yes       |
| 1   | Quil    | keyboard_nav_ready  | yes       |
| 2   | Linen   | nonblocking_ready   | yes       |
| 3   | Atlas   | overlay_available   | yes       |
| 4   | Bell    | ready               | yes       |
| 5   | Collar  | ready               | yes       |
| 6   | Mesh    | ready               | yes       |

All 7 apps are keyboard-ready and available.  Pointer/slot2 mouse deferred,
SilkBar palette ABI blocker documented.

## Markers Emitted

| Marker                  | Meaning                                 |
|-------------------------|-----------------------------------------|
| [launcher.open]         | count=N selected=N ok=N (palette opened)|
| [launcher.row]          | idx=N app=NAME status=NAME available=N  |
| [launcher.nav]          | old=N new=N count=N (selection moved)   |
| [launcher.exec]         | idx=N app=NAME ok=N reason=...          |
| [launcher.close]        | ok=N reason=... (palette closed)        |
| [launcher.proof]        | stage=N action=NAME ok=N reason=...      |
| [launcher.proof.done]   | ok=N                                    |

## Proof Gate

Activated by: `SEXOS_APP_LAUNCHER_PROOF=1`

5 stages:

| Stage | Action       | Verification                              |
|-------|--------------|-------------------------------------------|
| 1     | open         | Palette opens, launcher view visible      |
| 2     | nav_down     | Navigate down through all 7 app rows      |
| 3     | nav_up       | Navigate back up 3 positions              |
| 4     | exec         | Execute selected app (Spindle, idx 0)     |
| 5     | close        | Palette closes cleanly                    |

## Implementation Notes

- All launcher markers emitted in existing palette functions:
  - `toggle_command_palette()` — open and close markers
  - `palette_select_next()` / `palette_select_prev()` — nav markers
  - `palette_execute_selected()` — exec marker (for idx < 7 only)
- The proof function `maybe_run_app_launcher_proof()` is a bounded
  fire-and-forget proof that runs once at boot when enabled.
- No new UI, no duplicate app list, no behavior changes.
- App status labels come from existing `palette_item_status()` function.

## Build

```
SEXOS_APP_LAUNCHER_PROOF=1 ./scripts/entrypoint_build.sh
```

Or via the daily-driver proof profile:
```
SEXOS_APP_LAUNCHER_PROOF=1 ./scripts/run_daily_driver_proof.sh
```

## Runtime

```
timeout 30s qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku \
  -cdrom ./sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci -device usb-kbd,bus=xhci.0 \
  -serial file:/tmp/sexos_app_launcher_v1.log \
  -display none -no-reboot -no-shutdown || true
```

## Verification Results

| Metric          | Expected | Actual |
|-----------------|----------|--------|
| launcher.open   | 1        | 1      |
| launcher.row    | 7        | 7      |
| launcher.nav    | >=6      | 12     |
| launcher.exec   | 1        | 1      |
| launcher.close  | 1        | 1      |
| launcher.proof  | 15       | 15     |
| launcher.proof.done | 1   | 1      |
| Faults          | 0        | 0      |

## Files Changed
- servers/silk-shell/src/main.rs — added launcher markers in palette functions, launcher proof function, proof gate wiring
- docs/handoff/APP_LAUNCHER_V1.md — this handoff

## Notes
- No kernel/ABI/USB/input/display/pointer edits
- No broad refactor
- No blocking waits
- Existing command palette commands preserved
- App launcher is the existing palette's app subset — no duplication
