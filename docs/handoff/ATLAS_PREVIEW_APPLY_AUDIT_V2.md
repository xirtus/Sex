# ATLAS_PREVIEW_APPLY_AUDIT_V2

## Scope
- Atlas preset/accent preview marker audit only.
- No renderer or apply-path redesign.

## Files
- servers/silk-shell/src/main.rs

## Change
- In `maybe_run_atlas_preview_proof()`, normalized color formatting for marker stability:
  - from `color={:#x}`
  - to `color={:#010x}`
- Preserved marker contract:
  - `[atlas.preview] preset=N accent=N color=0xN ok=N`
  - `[atlas.preview.proof.done] ok=N`

## Proof
- Build gate: PASS
- Daily-driver gate: PASS target remains 18/18 with faults=0

## Notes
- Marker-only formatting polish; no behavior change.
