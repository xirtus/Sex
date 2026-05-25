# LINEN_SEXFILES_BASELINE_RESCAN_V1

Date: 2026-05-25
Baseline commit: `f284eec2a5160fcc90c1f5900520b19318071d22` (`f284eec2`)

## Current tags
- `linen-diskfs-persistence-100-current-tier-v1`
- `sexfiles-diskfs-100-current-tier-v1`
- `sexdrive-storage-100-current-tier-v1`
- `linen-sexfiles-100-current-tier-v1`
- `silk-de-100-current-tier-v1`
- `sexnet-real-internet-100-current-tier-v1`
- `atlas-overview-100-current-tier-v1`

## Repo cleanliness
- `git status --short`: `?? codex/`
- No tracked dirty files detected in baseline preflight.
- Untracked `.bak` files: none observed in this rescan.

## Proof commands run
- `./scripts/entrypoint_build.sh`
- `./scripts/run_daily_driver_proof.sh /tmp/linen_sexfiles_baseline_rescan_v1.log`
- `./scripts/daily_driver_master_gate.sh /tmp/linen_sexfiles_baseline_rescan_v1.log | tee /tmp/linen_sexfiles_baseline_rescan_v1_gate.txt`
- Focused scan:
  - `rg -n "linen|sexfiles|diskfs|sexdrive|storage|manifest|reboot|persist|fsync|flush|SLOT_STORAGE|SLOT_BLOCK|vfs|ramfs|disk|FINAL:|FAIL|#PF|#GP|panic|KERNEL PANIC|fault.kill" /tmp/linen_sexfiles_baseline_rescan_v1.log /tmp/linen_sexfiles_baseline_rescan_v1_gate.txt`

## Daily-driver final result
- `FINAL: PASS (255 gates proved, 113 skipped, 0 faults)`

## Fault scan
- `faults_zero PASS` and no `#PF/#GP/panic/KERNEL PANIC/fault.kill` flagged by final gate classification.

## Gate inventory (Linen/SexFiles/SexDrive/storage current-tier focus)
| Gate | Enablement condition | PASS criteria | FAIL criteria | SKIP criteria | Baseline status |
|---|---|---|---|---|---|
| `linen_persist_readback` | marker-driven | `linen.persist.readback.proof.done ok=1` or truth marker | none in this run | no proof markers | PASS |
| `linen_diskfs_fixed_object_save_load` | AP2 begin marker | AP2 select/read-match/done markers | timeout/fault/fail/missing mandatory AP2 markers | AP2 not triggered | SKIP |
| `linen_diskfs_reboot_restore` | AP3 begin marker | AP3 restore markers complete | timeout/fault/fail/missing AP3 markers | AP3 not triggered | SKIP |
| `linen_diskfs_metadata_persistence` | AP4 begin marker | AP4 metadata markers complete | timeout/fault/fail/contradicting markers | AP4 not triggered | SKIP |
| `linen_diskfs_negative_classifications` | AP5 begin marker | AP5 negative detection markers complete | timeout/fault/fail/missing negative markers | AP5 not triggered | SKIP |
| `sexfiles_diskfs_bridge_fixed_object_rw` | AP2 begin marker | IOQ-ready + select.ok + read.match + done | timeout/fault/fail/missing mandatory AP2 markers | AP2 not triggered | SKIP |
| `sexfiles_diskfs_bridge_multi_object_rw` | AP3 begin marker | linen/quil match + proof object intact + done | timeout/fault/fail/missing AP3 markers | AP3 not triggered | SKIP |
| `sexfiles_diskfs_bridge_reboot_persistence` | AP4 write/read begin markers | AP4 write/read match + done under correct phase | timeout/fault/fail/phase violations | AP4 not triggered | SKIP |
| `sexfiles_diskfs_bridge_negatives` | AP5 negative begin markers | expected negative detection + done | timeout/fault/fail/missing expected negative detection | AP5 not triggered | SKIP |
| `sexfiles_diskfs_bridge_flush_fsync_honest` | AP6 flush begin marker | explicit honest skip classification markers | false durability claims/fail markers/faults | AP6 not triggered | SKIP |
| `sexdrive_storage_ioq_ready` | storage AP2 profile | IOQ-ready storage marker path | profile fail markers | profile not requested | SKIP |
| `sexdrive_storage_single_block_rw` | storage AP3 profile | AP3 read/write roundtrip markers | profile fail markers | profile not requested | SKIP |
| `sexdrive_storage_multiblock_rw` | storage AP4 profile | AP4 multiblock markers | profile fail markers | profile not requested | SKIP |
| `sexdrive_storage_reboot_persistence` | storage AP5a profile | reboot persistence markers | profile fail markers | profile not requested | SKIP |
| `sexdrive_storage_flush_durability` | storage AP5b profile | completed flush proof | false durability/fail markers | profile not requested | SKIP |
| `sexdrive_storage_negatives` | storage AP6 profile | negative-path markers | profile fail markers | profile not requested | SKIP |

## Already-passing (evidence from this baseline run)
- `linen_nonblocking PASS`
- `linen_detail PASS`
- `linen_object_workflow PASS`
- `linen_object_persist PASS`
- `linen_object_schema PASS`
- `linen_search_bridge PASS`
- `linen_persist_readback PASS` (`durable=0 sync=0` model claim)
- Whole-run summary: `FINAL PASS`, `0 faults`

## Honest SKIP (not enabled in default daily profile)
- Linen AP2/AP3/AP4/AP5 DiskFS current-tier proof lanes
- SexFiles AP2/AP3/AP4/AP5/AP6 bridge lanes
- SexDrive AP2/AP3/AP4/AP5a/AP5b/AP6 storage lanes
- SKIP reasons are explicit “not triggered / not requested / profile not enabled” gate outputs.

## Real FAIL
- None in this baseline run (`FAIL gates: 0`).

## False-fail / stale-gate suspects
- None observed in this run for Linen/SexFiles/SexDrive current-tier gates.
- Current behavior is explicit-sentinel/profile gated with honest SKIP on default daily lane.

## Overclaim risks
- Existing closeout tags/docs can be misread as universal proof; this baseline confirms current-tier AP lanes were not executed in default profile.
- Keep strict non-claims unless AP-lane markers are present:
  - general filesystem semantics
  - POSIX semantics
  - crash/power-loss durability
  - journaling completeness
  - true flush/FUA durability
  - dynamic path IPC/generalized allocator-backed FS

## Current percentage estimate (evidence-conservative)
- SexFiles/DiskFS current-tier: **68-78%**
- Linen object flow current-tier: **82-90%**
- Combined Linen/SexFiles current-tier: **74-84%**
- Gap to 100%: **16-26%** (mainly AP2+ storage bridge and persistence-negative lanes not freshly re-proven in this baseline run)

## Recommended next autopilot ladder status
1. `LINEN_SEXFILES_BASELINE_RESCAN_V1` -> **KEEP (completed now)**
2. `SEXFILES_DISKFS_FIXED_OBJECT_CONTRACT_LOCK_V1` -> **KEEP**
3. `SEXFILES_DISKFS_BRIDGE_STRICT_PROOF_V1` -> **KEEP**
4. `LINEN_DISKFS_DIRECT_SAVE_LOAD_PROOF_V1` -> **MERGE** (coordinate with AP2 evidence refresh)
5. `LINEN_REBOOT_RESTORE_CURRENT_TIER_V1` -> **KEEP**
6. `SEXFILES_NEGATIVE_BOUNDS_AND_AUTH_PROOF_V1` -> **KEEP**
7. `LINEN_OBJECT_UX_CURRENT_TIER_PROOF_V1` -> **MERGE** (many UX gates already PASS; refresh only missing current-tier linkage)
8. `LINEN_SEXFILES_100_CURRENT_TIER_RELEASE_V1` -> **NEEDS FIX** (blocked on missing fresh AP-lane proofs and overclaim guard)

## STOP FIRST boundaries for remaining work
- No kernel edits unless STOP FIRST.
- No sex-pdx ABI/protocol edits unless STOP FIRST.
- No SexDrive durability-claim changes without explicit AP5b/AP6 proof execution.
- No filesystem-generalization claims beyond fixed-object/current-tier proof scope.

## Files changed
- `docs/handoff/LINEN_SEXFILES_BASELINE_RESCAN_V1.md` (created)
- No code changes in this mission.

## Contract lock follow-up (2026-05-25)
- Locked contract doc: `docs/handoff/SEXFILES_DISKFS_FIXED_OBJECT_CONTRACT_LOCK_V1.md`
- Status: `SEXFILES_DISKFS_FIXED_OBJECT_CONTRACT_LOCK_V1` complete (docs lock, no runtime behavior change).
- Next autopilot: `SEXFILES_DISKFS_BRIDGE_STRICT_PROOF_V1`.
