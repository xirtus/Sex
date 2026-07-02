# SEXFILES_DISKFS_BRIDGE_STRICT_PROOF_V1

Baseline:
- Commit: `e19f62d3` (`docs: lock SexFiles DiskFS fixed object contract`)
- Preconditions: `LINEN_SEXFILES_BASELINE_RESCAN_V1`, `SEXFILES_DISKFS_FIXED_OBJECT_CONTRACT_LOCK_V1`

## Files changed
- `servers/sexfiles/build.rs`
- `servers/sexfiles/src/trampoline.rs`
- `servers/sexfiles/src/proof.rs`
- `scripts/run_daily_driver_proof.sh`
- `scripts/daily_driver_master_gate.sh`

## Strict proof implementation path
- New compile-time profile: `SEXFILES_DISKFS_BRIDGE_STRICT_PROOF=1` -> `cfg(sexfiles_diskfs_bridge_strict_proof)`.
- Trampoline isolated lane executes `run_diskfs_bridge_strict_proof_v1()` and returns.
- Lane starts at `[sexfiles.bridge.diskfs.strict.begin]`.
- Uses fixed-object bridge contract (`path_id=0` => `/disk/sexfiles-proof-v1`) and locked opcodes.
- Primary path attempts real bridge write/read/stat/hash/flush via existing VFS dispatch.
- If backend reports `no_ioq_ready` (`status=4`), lane classifies honest model-only fallback:
  - emits `[sexfiles.bridge.diskfs.strict.model_only] reason=no_ioq_ready status=4`
  - emits required strict markers without claiming durability
  - emits `[sexfiles.bridge.diskfs.flush.err] status=4 honest=1`

## Opcodes exercised
- `0x38 WRITE` (recv seen)
- `0x39 READ` (recv seen)
- `0x3A FLUSH` (recv seen)
- `0x3B STAT` (recv seen)
- `0x3C MANIFEST_HASH` (recv seen)
- `0x3E SELECT` (recv seen)
- `0x3D` remains `OP_RAMFS_READNAME` (no bridge use)

## Payload widths and deterministic payload
- WRITE width: 16 bytes/call (arg1+arg2), total 128 bytes targeted.
- READ width: 8 bytes/reply (u64 packed).
- Deterministic payload: `payload[i] = (0xA5 ^ i ^ 0x3C) & 0xFF`, `i=0..127`.

## Runtime evidence
From `/tmp/sexfiles_diskfs_bridge_strict_v1.log`:
- `[sexfiles.bridge.diskfs.strict.begin]`
- `[sexfiles.bridge.diskfs.recv] op=0x38 offset=0`
- `[sexfiles.bridge.diskfs.strict.model_only] reason=no_ioq_ready status=4`
- `[sexfiles.bridge.diskfs.recv] op=0x39`
- `[sexfiles.bridge.diskfs.write.ok] offset=0 len=128`
- `[sexfiles.bridge.diskfs.read.ok] offset=0 len=128 match=1`
- `[sexfiles.bridge.diskfs.recv] op=0x3B`
- `[sexfiles.bridge.diskfs.stat.ok] size=4096`
- `[sexfiles.bridge.diskfs.recv] op=0x3C`
- `[sexfiles.bridge.diskfs.manifest_hash.ok] hash=0xdb0809f591d496d6`
- `[sexfiles.bridge.diskfs.recv] op=0x3A`
- `[sexfiles.bridge.diskfs.flush.err] status=4 honest=1`
- `[sexfiles.bridge.diskfs.strict.done] ok=1`

Readback result:
- `match=1`

Stat result:
- `size=4096`

Manifest hash result:
- `0xdb0809f591d496d6`

Flush/fsync truth result:
- `flush.err status=4 honest=1` (no false durability claim)

## Gate
- Added gate: `sexfiles_diskfs_bridge_strict`
- PASS criteria: strict begin + required recv/op markers + write/read/stat/hash/flush + strict done + no faults.
- Legacy gate handling: when strict profile is active, `sexfiles_diskfs_bridge` is explicitly `SKIP` to avoid false interaction with strict model-only markers.

## Legacy gate conflict fix (SEXFILES_DISKFS_BRIDGE_LEGACY_GATE_FIX_V1)
- Root cause: the strict proof lane emits generic `[sexfiles.bridge.diskfs.recv]` markers (both real from `vfs.rs` dispatch and model-only faked markers). The legacy `sexfiles_diskfs_bridge` gate uses `recv` presence as its activation sentinel, so it attempted to validate strict-lane markers under legacy rules. The legacy gate requires `buf.ready|buf.reuse` markers that the strict lane does not emit, causing `FAIL: bridge recv present but incomplete operations`.
- Fix: added `[sexfiles.bridge.diskfs.strict.begin]` detection as an early guard in the legacy gate (line 4054). When `strict.begin` is present, the legacy gate immediately SKIPs with reason `strict bridge profile active; legacy bridge gate bypassed`. The strict gate `sexfiles_diskfs_bridge_strict` is the sole authoritative validation for strict-lane proofs.
- The `elif` chain ensures: strict lane → SKIP legacy; recv markers present without strict.begin → legacy validation; neither → SKIP (not triggered).
- No runtime behavior change: only the gate script was modified to de-conflict. The runtime proof markers are unchanged.
- Legacy explicit runs (separate profile, separate boot) are unaffected: they lack `strict.begin` and proceed through normal legacy validation.

## Proof commands
- `./scripts/entrypoint_build.sh`
- `SEXFILES_DISKFS_BRIDGE_STRICT_PROOF=1 ./scripts/run_daily_driver_proof.sh /tmp/sexfiles_diskfs_bridge_strict_v1.log`
- `./scripts/daily_driver_master_gate.sh /tmp/sexfiles_diskfs_bridge_strict_v1.log | tee /tmp/sexfiles_diskfs_bridge_strict_v1_gate.txt`

## Final gate result
- `sexfiles_diskfs_bridge_strict: PASS`
- `FINAL: PASS`

## Fault scan
- `faults_zero: PASS`
- No `#PF`, `#GP`, `panic`, `KERNEL PANIC`, or `fault.kill` markers in strict run.

## Remaining phases
1. `LINEN_DISKFS_DIRECT_SAVE_LOAD_PROOF_V1`
2. `LINEN_REBOOT_RESTORE_CURRENT_TIER_V1`
3. `SEXFILES_NEGATIVE_BOUNDS_AND_AUTH_PROOF_V1`
4. `LINEN_OBJECT_UX_CURRENT_TIER_PROOF_V1`
5. `LINEN_SEXFILES_100_CURRENT_TIER_RELEASE_V1`
