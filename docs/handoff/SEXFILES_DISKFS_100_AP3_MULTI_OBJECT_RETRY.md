# SexFiles DiskFS 100 AP3.1 Multi-Object Profile Retry

## 1. Files Changed

| File | Change |
|------|--------|
| `servers/sexfiles/src/proof.rs` | Added `[sexfiles.diskfs100.ap3.done] ok=1` marker at end of `run_diskfs_multi_object_proofs()` |
| `servers/sexfiles/build.rs` | Gated `sexfiles_diskfs_multi_object_proof` cfg behind `SEXFILES_DISKFS_100_AP3_PROOF=1` |
| `scripts/run_daily_driver_proof.sh` | Added `SEXFILES_DISKFS_100_AP3_PROOF` env export (default 0) |
| `scripts/daily_driver_master_gate.sh` | Added `sexfiles_diskfs_bridge_multi_object_rw` gate |
| `sexos_build_spec.toml` | Removed hardcoded `--cfg sexfiles_diskfs_multi_object_proof` rustflag |
| `docs/handoff/SEXFILES_DISKFS_100_AP3_MULTI_OBJECT_RETRY.md` | This document |

## 2. AP2 Regression Result

- Command: `DAILY_DRIVER_PROBE_SECONDS=180 SEXOS_STORAGE_100_PROOF=1 SEXFILES_DISKFS_100_PROOF=1 ./scripts/run_daily_driver_proof.sh`
- `sexfiles_diskfs_bridge_fixed_object_rw`: **PASS**
- `sexfiles_diskfs_bridge_multi_object_rw`: SKIP
- `faults_zero`: PASS
- FAIL gates: 0
- FINAL: PASS

## 3. AP3 Profile Command

```
DAILY_DRIVER_PROBE_SECONDS=180 SEXOS_STORAGE_100_PROOF=1 SEXFILES_DISKFS_100_AP3_PROOF=1 ./scripts/run_daily_driver_proof.sh
```

## 4. Object List

From `run_diskfs_multi_object_proofs()`:
- **linen** — path_id=1, write 128 bytes (8x16B chunks), read 128 bytes (16x8B chunks), verify match
- **quil** — path_id=2, write 128 bytes (8x16B chunks), read 128 bytes (16x8B chunks), verify match
- **sexfiles-proof** — path_id=0, read 8 bytes intact verify

## 5. Runtime Markers

| Marker | Count |
|--------|-------|
| `ap3.begin` | 1 |
| `ap3.object.write.begin` | 8 (all linen) |
| `ap3.object.write.ok` | 8 (all linen) |
| `ap3.object.read.begin` | 16 (all linen, off=0..120) |
| `ap3.object.read.ok` | 15 (off=120 missing) |
| `ap3.object.match` | 0 |
| `ap3.done` | 0 |
| `ap3.fail` | 0 |
| PKU/fault/panic markers | 22 |
| `cqe_timeout` | 0 |

**Last AP3 marker before fault**: `[sexfiles.diskfs100.ap3.object.read.begin] name=linen path_id=1 off=120 len=8`

**Fault sequence**:
1. Linen read at off=120 I/O completes (lba=2046, cid=1348)
2. SexFiles reply processing dispatches `disk.file.lookup.ok start_lba=2030` (quil path_id=2) — before linen read.ok is printed
3. Page fault in task id=2 (pd_id=2) at RIP 0x410063ac
4. PKU security violation in PD 1 (kernel) at 0x70000e0ffdf8
5. KERNEL PANIC

## 6. Gate Result

- `sexfiles_diskfs_bridge_multi_object_rw`: **FAIL** (fault marker in AP3 profile log)
- `faults_zero`: FAIL (FAULTS FOUND: panic KERNEL PANIC PAGE FAULT)
- Cascade failures: `input_freeze_no_faults` FAIL, `silk_de_contract_lock` FAIL
- FAIL gates: 4
- FINAL: FAIL

## 7. Default Result

- Command: `./scripts/run_daily_driver_proof.sh`
- `sexfiles_diskfs_bridge_fixed_object_rw`: SKIP
- `sexfiles_diskfs_bridge_multi_object_rw`: SKIP
- `faults_zero`: PASS
- FAIL gates: 0
- FINAL: PASS

## 8. Classification

**PKU/fault** — The AP3 multi-object proof fails with a PKU security violation during the transition from linen object (path_id=1) to quil object (path_id=2).

Evidence:
- All 8 linen writes complete (off=0..112, 16-byte chunks)
- All 15 linen reads through off=112 complete successfully
- Last linen read (off=120) I/O completes but reply processing transitions to quil lookup immediately
- Page fault in PD 2 and PKU violation in kernel intercept the transition
- Same PKU fault pattern previously isolated in commit `77349bb1`
- NOT a timeout: I/O completes normally, no cqe_timeout, no no_ioq_ready
- NOT a profile mismatch: AP3 profile runs correctly when enabled, gate wired properly

### Build spec fix

Additionally discovered: `sexos_build_spec.toml` had `rustflags = "--cfg sexfiles_diskfs_multi_object_proof"` hardcoded, causing the multi-object proof to run in every default build regardless of env. This was removed and gated behind `SEXFILES_DISKFS_100_AP3_PROOF=1` in build.rs. Default runs now correctly SKIP AP3.

## 9. Next AP Recommendation

**AP3.2 — VA/PD mapping audit before multi-object transition.**

The fault is not a timeout or I/O error. It is a PKU/VA mapping fault that occurs at the boundary between DiskFS object path_id=1 (linen) and path_id=2 (quil). The kernel-level page fault and PKU violation suggest that the quil object's VA region or PKU key is not correctly mapped for the SexFiles PD context at the moment of transition.

Recommended next steps (DO NOT PROCEED HERE — this is handoff only):
1. Audit PKU key assignments for objects at path_id=1 vs path_id=2
2. Verify VA map grants are complete before quil object operations
3. Consider per-object PKU key allocation/switch in DiskFS bridge
4. Consider explicit PKU key switch markers before multi-object transitions
5. STOP FIRST: kernel/VA-map work may be required
