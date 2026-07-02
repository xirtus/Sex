# LINEN_SEARCH_FILTER_MARKERS_V2

## Scope
- Shell-side local Linen search/filter proof markers only.
- No blocking open and no PDX wait path.

## Files
- servers/silk-shell/src/main.rs

## Change
- In `maybe_run_linen_search_filter_proof()`:
  - preserved:
    - `[linen.search.query] len=N ok=N`
    - `[linen.search.result] count=N selected=N`
  - tightened done marker to include filter mode:
    - `[linen.filter.proof.done] ok=N mode=kind_document`

## Proof
- Build gate: PASS
- Daily-driver gate: PASS target remains 18/18 with faults=0

## Notes
- Marker-only audit; no behavioral changes.
