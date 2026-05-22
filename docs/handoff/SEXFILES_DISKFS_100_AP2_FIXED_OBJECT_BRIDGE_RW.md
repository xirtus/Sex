# SEXFILES_DISKFS_100_AP2_FIXED_OBJECT_BRIDGE_RW

## 1) Files changed
- servers/sexfiles/src/proof.rs          — Added `run_diskfs100_ap2_proof()` function
- servers/sexfiles/src/trampoline.rs     — Wired AP2 proof gate via `cfg!(sexfiles_diskfs100_ap2_proof)`; early return isolates AP2 profile from multi-object proof
- servers/sexfiles/build.rs              — Emits `cargo:rustc-cfg=sexfiles_diskfs100_ap2_proof` conditionally on `SEXFILES_DISKFS_100_PROOF=1`
- scripts/run_daily_driver_proof.sh      — Exports `SEXFILES_DISKFS_100_PROOF=1`
- scripts/daily_driver_master_gate.sh    — Added `sexfiles_diskfs_bridge_fixed_object_rw` gate

## 2) Fixed prerequisite
- Legacy real IO READ probe in `apps/sexdrive/src/main.rs` was gated behind:
  `SEXOS_STORAGE_100_IO_READ_PROBE=1`
- DiskFS no-probe lane now has `cqe_timeout=0`, DiskFS block replies status=0,
  31 block replies, extent proof done.
- CQ poison fix: commit `cfb8c8f9 fix(storage): gate legacy IO read probe`

## 3) Object identity
- Fixed object: `sexfiles-proof-v1`
- Path: `/disk/sexfiles-proof-v1`
- SELECT path_id: `0`

## 4) Payload formula
- `byte[i] = (0xC7 ^ i ^ 0x55) & 0xFF` for i in 0..128

## 5) Write/read chunk counts
- 8 write chunks × 16 bytes = 128 bytes written
- 8 read chunks × 16 bytes = 128 bytes read
- Byte-for-byte comparison: all 128 bytes match

## 6) Runtime markers (AP2 isolated profile: SEXOS_STORAGE_100_PROOF=1 SEXFILES_DISKFS_100_PROOF=1, 120s probe)
```
[sexfiles.diskfs100.ap2.begin]          object=sexfiles-proof-v1 bytes=128
[sexfiles.diskfs100.ap2.select.ok]      object=sexfiles-proof-v1
[sexfiles.diskfs100.ap2.write.chunk]    off=0..112 len=16 ok=1  (×8)
[sexfiles.diskfs100.ap2.read.chunk]     off=0..112 len=16 ok=1  (×8)
[sexfiles.diskfs100.ap2.read.match]     bytes=128 ok=1
[sexfiles.diskfs100.ap2.done]           ok=1
```
No multi-object proof markers present (profile isolation active).
No PKU violation markers present.

## 7) cqe_timeout
- **ABSENT**: 0 occurrences in AP2 profile log.

## 8) Gate result
- **AP2 isolated profile**: `sexfiles_diskfs_bridge_fixed_object_rw = PASS`
  ("IOQ-ready + select.ok + read.match ok=1 + done ok=1"), `faults_zero = PASS`,
  FAIL gates: 0, FINAL: PASS
- **Default profile**: `sexfiles_diskfs_bridge_fixed_object_rw = SKIP`
  (cfg not active, no ap2.begin marker), FAIL gates: 0, FINAL: PASS

## 9) Default result
- Default profile: 0 FAIL gates, FINAL PASS.
- AP2 cfg is conditionally set only when `SEXFILES_DISKFS_100_PROOF=1`.
- Default build does NOT set the cfg; AP2 proof does not run.
- Multi-object proof remains in its own profile/path (not affected by AP2 isolation).

## 10) Profile isolation
- Root cause: `build.rs` unconditionally emitted `rustc-cfg=sexfiles_diskfs100_ap2_proof`,
  causing AP2 proof to run in all builds. In the AP2 profile, execution continued past
  AP2 into `run_diskfs_multi_object_proofs()`, which triggered a PKU violation on Quil
  object write — unrelated to AP2 and pre-existing.
- Fix (build.rs): `rustc-cfg=sexfiles_diskfs100_ap2_proof` is now conditional on
  `SEXFILES_DISKFS_100_PROOF=1`.
- Fix (trampoline.rs): After `run_diskfs100_ap2_proof()` returns, the AP2 profile
  exits early with `return;`, preventing multi-object proof execution.
  The comment marker `[sexfiles.diskfs100.ap2.profile.done] isolated=1` documents
  the isolation point.
- Multi-object PKU violation is deferred to AP3 and no longer poisons AP2.

## 11) Non-claims
- no Linen
- no generic VFS path claims
- no directory claims
- no fsync/power-loss durability claims

## 12) Updated ladder
- AP1 reality audit: PASS
- AP2 fixed-object bridge RW: **PASS** (128-byte write/read/match verified against NVMe; profile isolated)
- AP3 multi-object pending (pre-existing PKU violation deferred from AP2)
- AP4 reboot persistence pending
- AP5 negatives pending
- AP6 flush/fsync honest classification pending
- AP7 closeout pending
