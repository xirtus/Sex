# LINEN SEXFILES 100 CURRENT TIER RELEASE V1

## Date
2026-05-25

## Status: PASS

The Linen/SexFiles current-tier 100% proof chain is closed with all
proven components in place and all limitations honestly classified.

---

## 1. Current-Tier Claim

Linen presents an honest bounded fixed-object UX over SexFiles DiskFS bridge.

### Proven (commits with PASS gates)
| Commit | Description | Gate |
|--------|------------|------|
| `badb08e9` | Direct save/load: 128-byte payload roundtrip via pdx_storage_sync | `linen_diskfs_direct` |
| `b5191e70` | Reboot restore: honestly deferred (no_ioq_ready/model_only) | `linen_reboot_restore_current_tier` |
| `bac1be37` | Negative bounds/auth: 7 rejection categories proven at SexFiles level | `sexfiles_diskfs_negative_bounds_auth` |
| `2af9799c` | Object UX: honest bounded classification, 0 overclaims | `linen_object_ux_current_tier` |

### Contract
```
fixed_object=/disk/sexfiles-proof-v1
object_size=4096
path_ids=0..2
max_read=8
max_write=16
```

### Route
```
Linen → SLOT_STORAGE(1) → SexFiles → DiskFS → SLOT_BLOCK(15) → SexDrive → NVMe
```

### Classification Markers
```
[linen.object_ux.current_tier.done] ok=1
[linen.object_ux.truth] linen_presents=honest_bounded_fixed_object_ux overclaims=0
                         proves=save_load+bounds_auth defers=reboot_restore
                         denies=posix+filesystem+durability ok=1
[linen.object_ux.proven] save_load=1 bounds_auth=1 ok=1
[linen.object_ux.contract] fixed_object=/disk/sexfiles-proof-v1 path_ids=0..2
                           object_size=4096 max_read=8 max_write=16
```

---

## 2. Explicit Non-Claims (Honest Denials)

| Capability | Status | Reason |
|-----------|--------|--------|
| POSIX filesystem | denied (0) | Fixed-object tier only |
| Directory tree | denied (0) | No directory semantics |
| Rename | denied (0) | Not in contract |
| Delete | denied (0) | Not in contract |
| Durability | denied (0) | no_ioq_ready, model_only fallback |
| Power-loss persistence | denied (0) | Not proven |
| Journal durability | denied (0) | Not in current tier |
| Reboot restore | deferred | AP3 path exists, blocked by dispatch timing |
| General filesystem | denied (0) | Bounded fixed-object UX only |
| Linen→SLOT_BLOCK | denied (0) | Route violation prevented |
| Linen→SexDrive | denied (0) | Route violation prevented |

---

## 3. Proof Commands

### Individual proofs:
```bash
# Direct save/load
SEXOS_LINEN_DISKFS_DIRECT_PROOF=1 ./scripts/run_daily_driver_proof.sh /tmp/linen_direct.log

# Reboot restore classification
LINEN_REBOOT_RESTORE_CURRENT_TIER_PROOF=1 ./scripts/run_daily_driver_proof.sh /tmp/linen_reboot.log

# Negative bounds/auth
SEXFILES_DISKFS_NEGATIVE_BOUNDS_AUTH_PROOF=1 ./scripts/run_daily_driver_proof.sh /tmp/sexfiles_neg.log

# Object UX classification
LINEN_OBJECT_UX_CURRENT_TIER_PROOF=1 ./scripts/run_daily_driver_proof.sh /tmp/linen_ux.log
```

### Release gate verification:
```bash
LINEN_OBJECT_UX_CURRENT_TIER_PROOF=1 ./scripts/run_daily_driver_proof.sh /tmp/linen_release.log
./scripts/daily_driver_master_gate.sh /tmp/linen_release.log | grep linen_sexfiles_100_current_tier_release
```

---

## 4. Gate Result

```
linen_sexfiles_100_current_tier_release  PASS  current-tier release: all markers present, honest denials verified, 0 overclaims
```

---

## 5. Remaining Future Tiers

1. **IOQ readiness** — real block device backing for DiskFS (replaces model_only)
2. **True two-boot reboot restore** — preserved NVMe image across QEMU boots
3. **Durability/power-loss proof** — with real flush/FUA/sync semantics
4. **Multi-object allocator** — beyond the current 3 fixed objects (path_ids 0..2)
5. **General filesystem semantics** — directories, rename, delete, dynamic paths
6. **Quil/Linen persistence unification** — shared object store across apps

---

## 6. Files Included in This Closeout

- `docs/handoff/LINEN_SEXFILES_100_CURRENT_TIER_RELEASE_V1.md` (this file)
- `scripts/daily_driver_master_gate.sh` — added `linen_sexfiles_100_current_tier_release` gate
- `scripts/run_daily_driver_proof.sh` — added env var (if changed)

## 7. Commit

```
docs: close Linen SexFiles current-tier release
```
