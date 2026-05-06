# SEXFILES_REPLAY_RECOVERY_PROOF_V1

## Purpose
Add deterministic bounded replay/recovery proof for SexFiles journal records:
- apply committed metadata transactions
- ignore uncommitted transactions
- reject corrupt records via checksum validation

## Replay Algorithm (Implemented)
1. Scan bounded journal records.
2. Validate checksum for every record; reject immediately with deterministic `ERR_OVERFLOW` on mismatch.
3. Track transaction begin/commit state in fixed arrays (`DISKFS_JOURNAL_CAPACITY` bounded).
4. Collect only `ObjectMetadataUpdate` records whose `tx_id` has a commit marker.
5. Ignore metadata updates from uncommitted transactions.
6. Apply committed metadata updates in ascending `generation` order.
7. Restore object entries into a bounded object table snapshot.

## Files Changed
- `servers/sexfiles/src/backends/diskfs.rs`
  - added replay structures/results (`ReplayOutcome`)
  - added bounded replay engine over journal records
  - added proof replay scenario synthesizing committed/uncommitted/corrupt streams
- `servers/sexfiles/src/proof.rs`
  - added replay proof runner and markers
- `servers/sexfiles/src/trampoline.rs`
  - added `SEXOS_SEXFILES_REPLAY_PROOF` gate hook
- `docs/handoff/SEXFILES_REPLAY_RECOVERY_PROOF_V1.md`

## Proof Gate / Markers
Gate:
- `SEXOS_SEXFILES_REPLAY_PROOF=1`

Markers:
- `[sexfiles.replay.proof.committed_applied]`
- `[sexfiles.replay.proof.uncommitted_ignored]`
- `[sexfiles.replay.proof.corrupt_rejected]`
- `[sexfiles.replay.proof.generation_order]`
- `[sexfiles.replay.proof.object_restored]`

Runtime evidence:
- All above markers emitted with `ok=1` in `.gate_master/serial.log`.

## Build / Runtime
- `cargo check --target sex-src/targets/x86_64-unknown-sexos.json -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem -p sexfiles`: PASS
- `./scripts/entrypoint_build.sh`: PASS
- `SEXOS_SEXFILES_REPLAY_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log`: PASS (`GREEN_MASTER`)

## Non-Goals Kept
- No kernel edits
- No `sex-pdx` ABI edits
- No snapshot implementation
- No POSIX/fsck semantics
- No app-visible raw journal/disk route

## Persistence Truth Level
- **RAM scaffold only**.
- Replay proof validates logic over bounded in-memory journal/object-table structures.
- No claim of real crash persistence on hardware-backed disk until block write/read + reboot recovery proof exists.

## Remaining Blockers
1. No real persistent block-device route in SexFiles->SexDrive for journal/object-table durability.
2. No reboot-time replay integration from persisted media.
3. No checkpoint record persistence and checkpoint selection logic.
4. No capability/revocation record replay path yet.
