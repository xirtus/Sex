# APP_LAUNCH_COMMANDS_V1 — Handoff

## Goal
Give Spindle (the keyboard control center) commands to list, explain, and
status-check apps.  Spindle cannot cross-PD launch — this is honestly reported.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `apps/spindle/src/main.rs` | Enhanced `apps` + `launch`, added `app-info` + `app-status` commands; proof gate | +94 |

## Commands Added

| Command | Behaviour |
|---------|-----------|
| `apps` | Lists 7 known apps with readiness status; emits [spindle.app.row] markers |
| `launch <app>` | Reports launch method (active or palette_owned); no cross-PD spawn |
| `app-info <app>` | Detailed info for one app: display name, kind, description, status, launch |
| `app-status` | Summary: 7 known, 1 active, 6 ready, 1 deferred |

## Markers (serial)
```
[spindle.app.command] name=NAME ok=N reason=...
[spindle.app.row] app=NAME status=NAME launch=NAME
[spindle.app.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_APP_LAUNCH_COMMANDS_PROOF=1
```
When enabled, `_start` auto-executes `apps`, `app-status`, `app-info spindle`,
`launch quil` at boot to emit markers without user keystrokes.

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `app_launch_commands`: PASS (19 rows)

## Safety / STOP FIRST
- ❌ No new kernel / ABI / USB / input / pointer / display changes
- ❌ No cross-PD spawn — launch is palette-owned, honestly reported
- ✅ Uses existing scrollback/push/dispatch infrastructure
- ✅ Static app mirror only; no PDX query to silk-shell

## Known Limitations
- Spindle cannot query silk-shell for live app state
- App mirror is static (7 entries)
- `app-info` id is always 1 (no real per-app PDX id)

## Future Follow-up
- Live app registry query via new PDX opcode to silk-shell
- Real launch via SLOT_SHELL or kernel-spawn
- Dynamic app install/uninstall tracking
