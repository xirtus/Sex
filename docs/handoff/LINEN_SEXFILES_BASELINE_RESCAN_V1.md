# LINEN_SEXFILES_BASELINE_RESCAN_V1

Date: 2026-05-25
Baseline commit: `f284eec2a5160fcc90c1f5900520b19318071d22` (`f284eec2`)
Current tags (`*100*current*tier*`):
- `linen-diskfs-persistence-100-current-tier-v1`
- `sexfiles-diskfs-100-current-tier-v1`
- `sexdrive-storage-100-current-tier-v1`
- `linen-sexfiles-100-current-tier-v1`
- `silk-de-100-current-tier-v1`
- `sexnet-real-internet-100-current-tier-v1`
- `atlas-overview-100-current-tier-v1`

## Repo cleanliness (Phase 0)
- `git status --short`: only `?? codex/` (untracked workspace dir)
- Tracked files dirty: no
- Untracked `.bak` files: none observed
- Safe to proceed into proof run: yes

## Commands run
- `./scripts/entrypoint_build.sh`
- `./scripts/run_daily_driver_proof.sh /tmp/linen_sexfiles_baseline_rescan_v1.log`
- `./scripts/daily_driver_master_gate.sh /tmp/linen_sexfiles_baseline_rescan_v1.log | tee /tmp/linen_sexfiles_baseline_rescan_v1_gate.txt`
- Focus scan:
  - `rg -n "linen|sexfiles|diskfs|sexdrive|storage|manifest|reboot|persist|fsync|flush|SLOT_STORAGE|SLOT_BLOCK|vfs|ramfs|disk|FINAL:|FAIL|#PF|#GP|panic|KERNEL PANIC|fault.kill" /tmp/linen_sexfiles_baseline_rescan_v1.log /tmp/linen_sexfiles_baseline_rescan_v1_gate.txt`

## Daily-driver result (Phase 3)
- Master gate FINAL: `PASS (255 gates proved, 113 skipped, 0 faults)`
- FAIL gates: `0`

## Fault scan
- `faults_zero PASS   0 fault markers`
- No `#PF`, `#GP`, `panic`, `KERNEL PANIC`, or `fault.kill` markers in this baseline run.

## Gate inventory (Linen/SexFiles/SexDrive/storage scope)

| Gate | Enablement condition | PASS criteria | FAIL criteria | SKIP criteria | Safe vs stale/false-enabled |
|---|---|---|---|---|---|
| `linen_persist_readback` | `linen.persist.readback.proof.done ok=1` or `linen.persist.truth` | done marker (or truth marker fallback) | n/a explicit fail branch | no markers | Safe, currently active and PASS |
| `linen_diskfs_direct` | `linen.diskfs.direct.begin` | write/read/done without fault/violation | panic/fault/violation/fake success/incomplete | begin absent | Stale reporting risk: SKIP path does not print row when begin absent |
| `linen_diskfs_fixed_object_save_load` | `linen.diskfs100.ap2.begin` | `ap2.content.match ok=1` + `ap2.done ok=1` | `cqe_timeout`/fault/`ap2.fail`/incomplete | AP2 begin absent | Safe gating, currently honest SKIP |
| `linen_diskfs_reboot_restore` | `ap3.write.begin` or `ap3.read.begin` | write/read phase markers complete and consistent | `cqe_timeout`/fault/`ap3.fail`/incomplete/mode violation | AP3 begin absent | Safe gating, currently honest SKIP |
| `linen_diskfs_metadata_persistence` | `ap4.meta.(audit|write|read).begin` | real metadata readback OR honest-skip classification markers | fail/fault/timeout/incomplete | AP4 begin absent | Safe gating, currently honest SKIP |
| `linen_diskfs_negative_classifications` | any `ap5.neg.*.begin` | expected neg detected/checked markers | fail/fault/timeout/begin without detection | AP5 begin absent | Safe gating, currently honest SKIP |
| `sexfiles_diskfs_bridge` | `sexfiles.bridge.diskfs.recv` | operation-complete marker set for exercised ops | fault/fake success/incomplete/no buffer marker | recv absent | Stale reporting risk: SKIP path does not print row when recv absent |
| `sexfiles_diskfs_bridge_fixed_object_rw` | `sexfiles.diskfs100.ap2.begin` | IOQ-ready + select/read.match/done markers | timeout/fail/missing required markers | AP2 begin absent | Stale reporting risk: SKIP path does not print row when AP2 begin absent |
| `sexfiles_diskfs_bridge_multi_object_rw` | `sexfiles.diskfs100.ap3.begin` | linen+quil+proof-object matches + done | timeout/fault/fail/incomplete | AP3 begin absent | Safe gating, currently honest SKIP |
| `sexfiles_diskfs_bridge_reboot_persistence` | `sexfiles.diskfs100.ap4.write.begin` or `.read.begin` | AP4 write/read completion markers | timeout/fault/fail/incomplete/mode violation | AP4 begin absent | Safe gating, currently honest SKIP |
| `sexfiles_diskfs_bridge_negatives` | AP5 begin markers | expected negative detection + done | fault/fail/incomplete | AP5 begin absent | Safe gating, currently honest SKIP |
| `sexfiles_diskfs_bridge_flush_fsync_honest` | `sexfiles.diskfs100.ap6.flush.begin` | explicit honest-skip classification markers | fault/fail/power-loss overclaim/incomplete | AP6 begin absent | Safe gating, currently honest SKIP |
| `sexdrive_storage_ioq_ready` | `sexdrive.storage100.ap2.begin` | IOQ ready marker exists | missing/failed readiness | AP2 begin absent | Safe gating, currently honest SKIP |
| `sexdrive_storage_single_block_rw` | `sexdrive.storage100.ap3.rw.begin` or storage proof mode | full AP3 write/read/match status=0 | missing/failure/nonzero status | proof not requested | Safe gating, currently honest SKIP |
| `sexdrive_storage_multiblock_rw` | `sexdrive.storage100.ap4.multi.begin` or storage proof mode | full AP4 multiblock marker set status=0 | missing/failure/nonzero status | proof not requested | Safe gating, currently honest SKIP |
| `sexdrive_storage_reboot_persistence` | AP5a write/read begin | phase-complete per boot role | mixed role in one log/failure | AP5a not requested | Safe gating, currently honest SKIP |
| `sexdrive_storage_flush_durability` | AP5b begin/skip markers | flush complete status=0 + done | fail/missing submit/missing completion | AP5b not requested or explicit skip | Safe gating, currently honest SKIP |
| `sexdrive_storage_negatives` | AP6 negative profile markers | expected negative detection markers | missing/fault | AP6 not triggered | Safe gating, currently honest SKIP |

## Already passing (baseline evidence)
| Item | Evidence |
|---|---|
| Linen nonblocking/detail/object workflows | `linen_nonblocking PASS`, `linen_detail PASS`, `linen_object_workflow PASS`, `linen_object_schema PASS` |
| Linen persist-readback current model | `linen_persist_readback PASS persist model (durable=0 sync=0)` |
| Storage phase-a/phase-b1 model gates | `storage_phasea PASS`, `storage_phaseb1 PASS` |
| Boot wiring for Linen/SexFiles/SexDrive slots | log markers show `SLOT_STORAGE` grants and `SLOT_BLOCK` grant to sexfiles |
| Fault-free baseline | `faults_zero PASS`, final gate `0 faults` |

## Honest SKIPs
| Item | Why SKIP |
|---|---|
| `linen_diskfs_fixed_object_save_load` | AP2 proof not triggered |
| `linen_diskfs_reboot_restore` | AP3 proof not triggered |
| `linen_diskfs_metadata_persistence` | AP4 proof not triggered |
| `linen_diskfs_negative_classifications` | AP5 proof not triggered |
| `sexfiles_diskfs_bridge_multi_object_rw` | AP3 proof not triggered |
| `sexfiles_diskfs_bridge_reboot_persistence` | AP4 proof not triggered |
| `sexfiles_diskfs_bridge_negatives` | AP5 proof not triggered |
| `sexfiles_diskfs_bridge_flush_fsync_honest` | AP6 proof not triggered |
| `sexdrive_storage_ioq_ready`..`sexdrive_storage_negatives` | storage AP2-AP6 proof profile not requested |

## Real FAILs
- None in this baseline run (`FAIL gates: 0`).

## False-fail / stale-gate suspects
- Reporting hygiene issue: `linen_diskfs_direct`, `sexfiles_diskfs_bridge`, and `sexfiles_diskfs_bridge_fixed_object_rw` set `SKIP` without calling `print_row` when not triggered. This can hide lane status from summaries and create stale interpretation risk even when behavior is technically SKIP.
- No active false FAIL observed in this run.

## Overclaim risks (current-tier boundary)
- Current run does not prove AP2/AP3/AP4/AP5/AP6 DiskFS storage lanes; any claim of complete current-tier 100% from this single run would overclaim.
- The run includes model/status passes (`storage_phasea`, `storage_phaseb1`, `linen_persist_readback`) but not explicit fixed-object/reboot/negative/flush proof profiles.
- Not allowed claims remain unproven here: general FS semantics, POSIX behavior, dynamic paths, delete/rename, crash consistency, journaling, true FLUSH/FUA durability, power-loss durability.

## Current percentage estimate (evidence-conservative)
- SexFiles/DiskFS current-tier: **70-85%** (baseline runtime healthy; explicit AP2-AP6 not rerun here)
- Linen object flow current-tier: **80-90%** (nonblocking/object/persist-model PASS; DiskFS AP2-AP5 not rerun)
- Combined Linen/SexFiles current-tier: **75-88%**
- Gap to 100% current-tier (for this baseline evidence set): **12-25%**

## Next autopilot ladder decision
1. `LINEN_SEXFILES_BASELINE_RESCAN_V1` — **KEEP** (completed by this handoff)
2. `SEXFILES_DISKFS_FIXED_OBJECT_CONTRACT_LOCK_V1` — **KEEP** (not proven in this run)
3. `SEXFILES_DISKFS_BRIDGE_STRICT_PROOF_V1` — **KEEP** (bridge AP2+ strict proof not rerun)
4. `LINEN_DISKFS_DIRECT_SAVE_LOAD_PROOF_V1` — **KEEP** (AP2 not rerun)
5. `LINEN_REBOOT_RESTORE_CURRENT_TIER_V1` — **KEEP** (AP3 not rerun)
6. `SEXFILES_NEGATIVE_BOUNDS_AND_AUTH_PROOF_V1` — **KEEP** (AP5/AP6 not rerun)
7. `LINEN_OBJECT_UX_CURRENT_TIER_PROOF_V1` — **MERGE** (many UX lanes already PASS, focus only missing current-tier deltas)
8. `LINEN_SEXFILES_100_CURRENT_TIER_RELEASE_V1` — **NEEDS FIX** (do only after AP2-AP6 reruns are freshly PASS/intentional SKIP with explicit evidence)

## STOP FIRST boundaries (remaining work)
- No kernel edits unless STOP FIRST.
- No `sex-pdx` ABI/protocol edits unless STOP FIRST.
- No SexDrive/NVMe durability behavior changes in this chain unless STOP FIRST.
- Gate-hygiene-only follow-up may edit `scripts/daily_driver_master_gate.sh` for missing SKIP row visibility, but only as tiny reporting fix.

## Files changed in this mission
- `docs/handoff/LINEN_SEXFILES_BASELINE_RESCAN_V1.md` (created)
