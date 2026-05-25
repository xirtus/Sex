# LINEN_DISKFS_DIRECT_SAVE_LOAD_PROOF_V1

Baseline:
- Commit: `8fcaf81d` (`sexfiles: prove DiskFS fixed object bridge`)
- Preconditions: `SEXFILES_DISKFS_FIXED_OBJECT_CONTRACT_LOCK_V1`, `SEXFILES_DISKFS_BRIDGE_STRICT_PROOF_V1`

## Files changed
- `servers/linen/src/main.rs`
- `scripts/daily_driver_master_gate.sh`
- `scripts/run_daily_driver_proof.sh`
- `docs/handoff/LINEN_DISKFS_DIRECT_SAVE_LOAD_PROOF_V1.md` (this file)

## Proof overview
Linen direct DiskFS save/load proof. Linen writes a deterministic 128-byte payload through the locked SexFiles DiskFS bridge using only `SLOT_STORAGE` (slot=1), then reads it back and verifies byte-for-byte match.

## Linen route attestation
- `SLOT_STORAGE` (slot=1) → SexFiles VFS → DiskFS fixed object bridge → `SLOT_BLOCK` → SexDrive → NVMe
- Linen never accesses `SLOT_BLOCK` (slot=15) directly: `uses_slot_block=0`
- Linen never calls SexDrive directly: `direct_sexdrive=0`
- Opcodes used: 0x38 (WRITE), 0x39 (READ), 0x3A (FLUSH), 0x3B (STAT), 0x3C (MANIFEST_HASH), 0x3E (SELECT)

## Write/read chunk contract
- WRITE: 8 chunks × 16 bytes via OP_DISKFS_WRITE (0x38), packed into arg1+arg2 u64 pair
- READ: 16 chunks × 8 bytes via OP_DISKFS_READ (0x39), each reply packed into one u64
- Payload: 128 bytes, Linen-specific header (object_id, kind, owner_pd, generation, flags, name) + deterministic tail xor pattern

## Explicit non-goals
- **No SLOT_BLOCK** access (uses_slot_block=0)
- **No direct SexDrive** calls (direct_sexdrive=0)
- **No durability/reboot claim** — this is a single-boot save/load roundtrip
- **No general filesystem claim** — fixed-object bridge only
- **No dynamic paths**, delete, rename, directories
- **No POSIX semantics**

## Gate
- Gate: `linen_diskfs_direct`
- PASS: begin + route attestation (uses_slot_block=0, direct_sexdrive=0) + write.ok bytes=128 + read.ok bytes=128 + read.match ok=1 + stat.ok size=4096 + done ok=1 + no faults
- FAIL: fault, slot_block violation, sexdrive violation, read mismatch, missing markers
- SKIP: begin absent

## Proof commands
- `./scripts/entrypoint_build.sh`
- `SEXOS_LINEN_DISKFS_DIRECT_PROOF=1 ./scripts/run_daily_driver_proof.sh /tmp/linen_diskfs_direct_save_load_v1.log`
- `./scripts/daily_driver_master_gate.sh /tmp/linen_diskfs_direct_save_load_v1.log | tee /tmp/linen_diskfs_direct_save_load_v1_gate.txt`

## Remaining phases
1. `LINEN_REBOOT_RESTORE_CURRENT_TIER_V1`
2. `SEXFILES_NEGATIVE_BOUNDS_AND_AUTH_PROOF_V1`
3. `LINEN_OBJECT_UX_CURRENT_TIER_PROOF_V1`
4. `LINEN_SEXFILES_100_CURRENT_TIER_RELEASE_V1`
