# PROOF_ENV_REGISTRY_REFRESH_V3

## Build + Gate Baseline
- `./scripts/entrypoint_build.sh`
- `./scripts/run_daily_driver_proof.sh <log>`

## Registry / Launcher / Palette
- `SEXOS_APP_REGISTRY_READONLY_PROOF`
- `SEXOS_APP_REGISTRY_FILTER_SORT_PROOF`
- `SEXOS_APP_REGISTRY_LAUNCH_INTENT_PROOF`
- `SEXOS_APP_LAUNCHER_HELP_PROOF`

## Linen / Bell / Atlas
- `SEXOS_LINEN_SEARCH_FILTER_PROOF`
- `SEXOS_BELL_FILTER_PROOF`
- `SEXOS_ATLAS_PREVIEW_PROOF`

## Spindle
- `SEXOS_SPINDLE_ALIASES_PROOF`
- `SEXOS_SPINDLE_APPS_REGISTRY_PROOF`

## Marker Family Additions in V4
- `app.registry.intent*`
- `linen.search.token`
- `launcher.help.row ... reason=...`
- `bell.filter.source ... reason=empty_ring` (zero-ring path)

## Notes
- Source changes must always be followed by full daily-driver gate run.
