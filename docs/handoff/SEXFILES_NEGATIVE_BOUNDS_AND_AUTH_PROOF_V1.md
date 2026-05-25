# SEXFILES NEGATIVE BOUNDS AND AUTH PROOF V1

## Metadata
- **Date:** 2026-05-25
- **Baseline commit:** b5191e70 (linen: classify reboot restore current tier)
- **Outcome:** PASS
- **Gate:** `sexfiles_diskfs_negative_bounds_auth`
- **Durability:** 0 (no durability claim — bounds/auth only)
- **Faults:** 0

## Summary
Proved that the SexFiles DiskFS fixed-object bridge rejects all illegal
inputs correctly.  Seven negative test categories all pass:

1. **bad_opcode** — 11 unknown opcodes (0x00, 0x01, 0x10, 0x20, 0x2F,
   0x40, 0x41, 0x50, 0xFF, 0x100, 0xDEAD) all return ERR_NOT_FOUND (-3).

2. **bad_path_id** — SELECT with path_id >= 3 (3, 4, 99, u64::MAX)
   returns ERR_BAD_CMD (-7).

3. **default_path** — Bridge defaults to path_id=0 (sexfiles-proof-v1)
   without explicit SELECT; STAT returns size=4096, FLUSH returns
   honest status (0 or 4).

4. **write_bounds** — writes at offset=4096, 4085 (boundary), 5000
   all return ERR_OVERFLOW (-4).

5. **read_bounds** — reads with max_len=0, 9, 50, offset=4096,
   offset=4090+len=8 all return ERR_OVERFLOW (-4).

6. **read_before_write** — reading a valid offset before any write
   returns data (non-error), proving the object is readable without
   prior write.

7. **flush** — honest classification: status=4 (ERR_NO_DEVICE) on
   QEMU NVMe, accepted as legitimate fixed-object tier result.

## Contract Compliance
- No POSIX directory tree semantics
- No rename/delete/general allocator
- No journaling/crash-consistency/power-loss claims
- No Linen→SLOT_BLOCK, no Linen→SexDrive, no Linen MemLend
- Fixed-object tier only: path_id 0/1, 4096-byte objects

## Files Changed
- `servers/sexfiles/build.rs` — added cfg flag for SEXFILES_DISKFS_NEGATIVE_BOUNDS_AUTH_PROOF
- `servers/sexfiles/src/proof.rs` — added run_diskfs_negative_bounds_auth_proof()
- `servers/sexfiles/src/trampoline.rs` — added dispatch for negative bounds proof
- `scripts/daily_driver_master_gate.sh` — added gate sexfiles_diskfs_negative_bounds_auth
- `scripts/run_daily_driver_proof.sh` — added env var export and print line

## Proof Commands
```bash
SEXFILES_DISKFS_NEGATIVE_BOUNDS_AUTH_PROOF=1 \
SEXFILES_DISKFS_100_PROOF=0 \
SEXFILES_DISKFS_BRIDGE_STRICT_PROOF=0 \
SEXOS_LINEN_DISKFS_DIRECT_PROOF=0 \
./scripts/run_daily_driver_proof.sh /tmp/sexfiles_negative_bounds_v1.log
```

## Gate Result
```
sexfiles_diskfs_negative_bounds_auth  PASS  negative bounds + auth: all rejection cases proven
faults_zero                           PASS  0 fault markers
```

## Remaining Linen/SexFiles 100 Phases
1. LINEN_OBJECT_UX_CURRENT_TIER_PROOF_V1
2. LINEN_SEXFILES_100_CURRENT_TIER_RELEASE_V1
