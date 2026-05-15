# LAUNCHER_STATUS_REASON_NORMALIZE_V1

## Scope
- Launcher help row marker normalization.
- Marker-only change in shell proof path.

## File
- servers/silk-shell/src/main.rs

## Change
- `maybe_run_app_launcher_help_proof()` now emits reason in row marker:
  - from: `[launcher.help.row] idx=N app=NAME key=NAME status=NAME`
  - to:   `[launcher.help.row] idx=N app=NAME key=NAME status=NAME reason=NAME`

## Notes
- Uses existing `palette_item_status()` reason text.
- No launcher behavior changes.
