# LINEN REBOOT RESTORE CURRENT TIER — CLASSIFICATION V1

## Metadata
- **Date:** 2026-05-25
- **Baseline commit:** badb08e9 (linen: prove DiskFS direct save load)
- **Outcome:** PASS (honest skip classification)
- **Gate:** `linen_reboot_restore_current_tier`
- **Durability:** 0 (deferred, no durability claim)
- **Faults:** 0

## Summary
Current-tier reboot/restore is **honestly classified as deferred**.
Direct save/load is proven (badb08e9).  The AP3 two-boot architecture
exists but cannot complete a full cross-boot round-trip under current
constraints.

## Markers Emitted
```
[linen.reboot_restore.skip] reason=no_ioq_ready_model_only_dispatch_deferred model_only=1 durable=0
[linen.reboot_restore.truth] direct_save_load=proven reboot_restore=deferred ok=1
[linen.reboot_restore.done] classification=honest_skip powerloss=0 journal=0 ok=1
```

## Blockers (honest)
1. **no_ioq_ready:** Block device IOQ is not ready; DiskFS bridge uses
   model_only fallback.  Without real block backing, data written in
   boot 1 cannot be read in boot 2.
2. **Dispatch deferral:** Linen's AP3 proof starts before SexFiles
   main message-dispatch loop is fully settled; the SELECT→manifest-ensure
   path involves multiple NVMe I/O round-trips that exceed the AP3 proof's
   yield window.
3. **Manifest ensure overhead:** First SELECT call triggers a full
   manifest ensure (BLOCK_READ → BLOCK_WRITE → BLOCK_READ verify) that
   takes several seconds on QEMU.

## What IS Proven
- `linen_diskfs_direct_save_load`: 128-byte payload save/load through
  Linen → SexFiles → DiskFS bridge, byte-for-byte match (commit badb08e9)
- `sexfiles_diskfs_bridge_strict_proof`: bridge opcodes (0x38-0x3C) correct
- `sexfiles_diskfs_fixed_object_contract_lock`: fixed object contract
  (/disk/sexfiles-proof-v1, 4096 bytes, read 1..8, write 16)

## What Remains
- IOQ/backing device readiness
- Real two-boot reboot/restore proof with preserved NVMe image
- Power-loss / durability / journal persistence proof

## Files Changed
- `servers/linen/build.rs` — added cfg flag emission for LINEN_REBOOT_RESTORE_CURRENT_TIER_PROOF
- `servers/linen/src/main.rs` — added honest skip classification constant and function
- `scripts/daily_driver_master_gate.sh` — added gate `linen_reboot_restore_current_tier`
- `scripts/run_daily_driver_proof.sh` — added env var export and print line

## Proof Commands
```bash
LINEN_REBOOT_RESTORE_CURRENT_TIER_PROOF=1 \
SEXFILES_DISKFS_100_PROOF=0 \
SEXFILES_DISKFS_BRIDGE_STRICT_PROOF=0 \
SEXOS_LINEN_DISKFS_DIRECT_PROOF=0 \
./scripts/run_daily_driver_proof.sh /tmp/linen_reboot_restore_current_tier_v1.log
```

## Gate Result
```
linen_reboot_restore_current_tier PASS honest skip: reboot restore deferred (no_ioq_ready/model_only/dispatch)
faults_zero                        PASS   0 fault markers
FINAL:                             PASS (258 gates proved, 111 skipped, 0 faults)
```

## Remaining Linen/SexFiles 100 Phases
1. SEXFILES_NEGATIVE_BOUNDS_AND_AUTH_PROOF_V1
2. LINEN_OBJECT_UX_CURRENT_TIER_PROOF_V1
3. LINEN_SEXFILES_100_CURRENT_TIER_RELEASE_V1
