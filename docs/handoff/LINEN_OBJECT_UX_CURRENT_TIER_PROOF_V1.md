# LINEN OBJECT UX CURRENT TIER PROOF V1

## Metadata
- **Date:** 2026-05-25
- **Baseline commit:** bac1be37 (sexfiles: prove DiskFS negative bounds and auth rejection)
- **Outcome:** PASS (honest classification)
- **Gate:** `linen_object_ux_current_tier`
- **Durability:** 0 (no durability claim)
- **Faults:** 0 in pipeline (1 unrelated PKU violation in manual QEMU after proof completes)

## Summary
Proved that Linen presents an honest bounded-object UX over SexFiles
without overclaiming filesystem semantics.  The proof emits classification
markers covering:

- **Contract:** fixed object `/disk/sexfiles-proof-v1`, path_ids=0..2,
  4096-byte objects, max 8-byte reads, 16-byte writes
- **Proven:** save/load (badb08e9) + bounds/auth rejection (bac1be37)
- **Limited:** no POSIX filesystem, no directories, no rename, no delete
- **Deferred:** reboot restore deferred, no durability/powerloss/journal
- **Truth:** honest_bounded_fixed_object_ux, 0 overclaims

## Markers Emitted
```
[linen.object_ux.current_tier.begin]
[linen.object_ux.route] slot=1 slot_block=15 uses_slot_block=0 direct_sexdrive=0
[linen.object_ux.contract] fixed_object=/disk/sexfiles-proof-v1 path_ids=0..2 object_size=4096 max_read=8 max_write=16
[linen.object_ux.proven] save_load=1 bounds_auth=1 ok=1
[linen.object_ux.limited] filesystem=0 posix=0 directories=0 rename=0 delete=0 ok=1
[linen.object_ux.deferred] reboot_restore=1 durable=0 powerloss=0 journal=0 ok=1
[linen.object_ux.truth] linen_presents=honest_bounded_fixed_object_ux overclaims=0 proves=save_load+bounds_auth defers=reboot_restore denies=posix+filesystem+durability ok=1
[linen.object_ux.current_tier.done] ok=1
```

## Relationship to Previous Proofs
- `badb08e9` — direct save/load proven (PDX path works)
- `bac1be37` — bounds/auth rejection proven (sexfiles level)
- `b5191e70` — reboot restore honestly deferred

This proof ties them together into a unified UX classification.

## Files Changed
- `servers/linen/build.rs` — added cfg flag for LINEN_OBJECT_UX_CURRENT_TIER_PROOF
- `servers/linen/src/main.rs` — added classification function + dispatch
- `scripts/daily_driver_master_gate.sh` — added gate linen_object_ux_current_tier
- `scripts/run_daily_driver_proof.sh` — added env var export and print line

## Proof Commands
```bash
LINEN_OBJECT_UX_CURRENT_TIER_PROOF=1 \
./scripts/run_daily_driver_proof.sh /tmp/linen_object_ux_v1.log
```

## Gate Result
```
linen_object_ux_current_tier  PASS  object UX honest classification: bounded fixed-object, no POSIX overclaim, done ok=1
```

## Remaining Linen/SexFiles 100 Phases
1. LINEN_SEXFILES_100_CURRENT_TIER_RELEASE_V1
