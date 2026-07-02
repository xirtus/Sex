# PROOF_ENV_REGISTRY_V2

## Core Gate Commands
- Build gate: `./scripts/entrypoint_build.sh`
- Runtime gate: `./scripts/run_daily_driver_proof.sh <log_path>`

## Shell / Launcher / Palette
- `SEXOS_APP_LAUNCHER_HELP_PROOF`
  - markers: `launcher.help.keys`, `launcher.help.row`, `launcher.help.proof.done`
- `SEXOS_APP_LAUNCHER_MULTI_EXEC_PROOF`
  - markers: `launcher.multi.proof`, `launcher.multi.exec`, `launcher.multi.focus`, `launcher.multi.proof.done`
- `SEXOS_COMMAND_PALETTE_DAILY_PROOF`
  - markers: `shell.palette.daily.proof.*`

## Linen / Registry
- `SEXOS_LINEN_SEARCH_FILTER_PROOF`
  - markers: `linen.search.query`, `linen.search.result`, `linen.filter.proof.done`
- `SEXOS_APP_REGISTRY_READONLY_PROOF`
  - markers: `app.registry.row`, `app.registry.readonly.proof.done`
- `SEXOS_APP_REGISTRY_FILTER_SORT_PROOF`
  - markers: `app.registry.filter`, `app.registry.sort`

## Bell / Atlas
- `SEXOS_BELL_FILTER_PROOF`
  - markers: `bell.filter.source`, `bell.filter.nav`, `bell.filter.proof.done`
- `SEXOS_ATLAS_PREVIEW_PROOF`
  - markers: `atlas.preview`, `atlas.preview.proof.done`

## Spindle
- `SEXOS_SPINDLE_ALIASES_PROOF`
  - markers: `spindle.alias.exec`, `spindle.alias.proof.done`
- `SEXOS_SPINDLE_APPS_REGISTRY_PROOF`
  - markers: `spindle.apps.registry.row`, `spindle.apps.registry.done`

## Notes
- Run full daily-driver proof after any source change, even if env-gated markers are target-only.
- Keep marker names stable and grep-friendly.
