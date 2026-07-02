# BELL_SOURCE_FILTER_DETAIL_V2

## Scope
- Keyboard Bell source filter proof markers only.
- Local event ring only; no Bell model redesign.

## Files
- servers/silk-shell/src/main.rs

## Change
- Audited `maybe_run_bell_filter_proof()` marker semantics.
- Kept source and nav markers:
  - `[bell.filter.source] source=local_ring count=N ok=N`
  - `[bell.filter.nav] old=N new=N ok=N`
- Tightened done marker to be derived from source/nav evidence:
  - `[bell.filter.proof.done] ok=N`

## Proof
- Build gate: PASS
- Daily-driver gate: PASS target remains 18/18 with faults=0

## Notes
- Marker-only/safety change; no event behavior redesign.
