# ATLAS_FINAL_CLOSEOUT_GATE_HYGIENE_AFTER_LINEN_AP4

## Scope
- Mission: Gate hygiene only for `atlas_overview_final_closeout` false FAIL during Linen AP4 metadata audit log replay.
- Constraints honored:
  - No kernel/runtime/server/app edits.
  - Gate script only.
  - Preserve strict PASS/FAIL once explicit Atlas final proof is truly requested.

## Backup Before Changes
- Snapshot created:
  - `/tmp/daily_driver_master_gate.sh.bak_atlas_final_closeout_gate_hygiene_after_linen_ap4`

## Root Cause
- `atlas_overview_final_closeout` was enabled by `silk.atlas.overview.final.begin/done` markers alone.
- In the Linen AP4 metadata audit log, those silk markers appeared incidentally (`callpath.enabled=1`, `begin`, `done`) even though the run was not an Atlas final closeout mission.
- Gate then enforced full Atlas subphase completeness and failed on missing phase `B`, producing an unrelated false FAIL for the AP4 mission.

## Exact Gate Change
- File: `scripts/daily_driver_master_gate.sh`
- Added a narrow enablement guard before evaluating Atlas final closeout:
  - Tracks dedicated explicit Atlas begin markers:
    - `[atlas.overview.final.begin]`
    - `[atlas.final.closeout.begin]`
  - Tracks silk begin marker separately:
    - `[silk.atlas.overview.final.begin]`
  - If log is a Linen AP4 metadata audit run (`[linen.diskfs100.ap4.meta.audit.begin]`) and only silk begin is present (no dedicated Atlas begin), the gate treats Atlas final closeout as not requested (SKIP).
- PASS/FAIL logic for explicit requested Atlas final closeout remains unchanged.

## AP4 Log Verification
Command:
```bash
./scripts/daily_driver_master_gate.sh /tmp/linen_diskfs_ap4_meta_audit.log \
  | grep -E "atlas_overview_final_closeout|linen_diskfs_metadata_persistence|faults_zero|FAIL gates|FINAL"
```

Result:
- `linen_diskfs_metadata_persistence PASS   honest skip: metadata is RamFS/session-only, not DiskFS-backed`
- `atlas_overview_final_closeout SKIP   final closeout proof not enabled or incomplete`
- `faults_zero PASS   0 fault markers`
- `FAIL gates: 0`
- `FINAL: PASS (262 gates proved, 105 skipped, 0 faults)`

## Default Regression
Commands:
```bash
./scripts/run_daily_driver_proof.sh
./scripts/daily_driver_master_gate.sh /tmp/sexos_daily_driver_proof.log \
  | grep -E "atlas_overview_final_closeout|linen_diskfs_metadata_persistence|faults_zero|FAIL gates|FINAL"
```

Result highlights:
- `atlas_overview_final_closeout SKIP   final closeout proof not enabled or incomplete`
- `linen_diskfs_metadata_persistence SKIP   AP4 metadata persistence proof not triggered`
- `faults_zero PASS   0 fault markers`
- `FAIL gates: 0`
- `FINAL: PASS (255 gates proved, 112 skipped, 0 faults)`

## Non-Effect On Explicit Atlas Final Proof
- When dedicated Atlas final begin marker(s) are present, gate remains strict:
  - If `final.done` and all required subphase markers are present: PASS.
  - If enabled but incomplete: FAIL.
- Change only blocks incidental Linen AP4 activation path from producing unrelated Atlas FAIL.
