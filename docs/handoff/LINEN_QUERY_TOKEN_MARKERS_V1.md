# LINEN_QUERY_TOKEN_MARKERS_V1

## Scope
- Add explicit token marker for local Linen query proof.

## File
- servers/silk-shell/src/main.rs

## Change
- In `maybe_run_linen_search_filter_proof()`, added:
  - `[linen.search.token] idx=0 value=doc ok=1`
- Existing query/result/done markers preserved.

## Notes
- Marker-only update; no open path or blocking behavior.
