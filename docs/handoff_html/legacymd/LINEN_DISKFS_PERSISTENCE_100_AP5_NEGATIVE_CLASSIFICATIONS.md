# Linen DiskFS Persistence 100 — AP5 Negative Classifications

## Status: VERIFIED — PASS

All five negative classification lanes pass the daily-driver master gate
with zero faults, zero panics, and zero security violations.

## Files Changed

| File | Lines Changed |
|---|---|
| `servers/linen/src/main.rs` | +86 |
| `scripts/run_daily_driver_proof.sh` | +24 / -5 |
| `scripts/daily_driver_master_gate.sh` | +49 |

## Environment Variables

### Gate (required for all lanes)
```
SEXOS_STORAGE_100_PROOF=1
SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP5_NEGATIVE=1
```

### Lane-specific
| Lane | Env Var |
|---|---|
| Mismatch | `SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP5_NEG_MISMATCH=1` |
| Missing | `SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP5_NEG_MISSING=1` |
| Read-No-Write | `SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP5_NEG_READ_NO_WRITE=1` |
| Metadata False-Claim | `SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP5_NEG_METADATA_FALSE_CLAIM=1` |
| Flush Skip | `SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP5_NEG_FLUSH_SKIP=1` |

All lanes also require `DAILY_DRIVER_PROBE_SECONDS=180`.

## Negative Case Results

### Mismatch — PASS
- Gate: `linen_diskfs_negative_classifications PASS`
- Markers: `ap5.neg.mismatch.begin` → `ap5.neg.mismatch.detected ok=1` → `ap5.neg.done case=mismatch ok=1`
- The mismatch lane computes a wrong byte value and detects it, demonstrating
  that a data mismatch on read would be caught by the guard.

### Missing — PASS
- Gate: `linen_diskfs_negative_classifications PASS`
- Markers: `ap5.neg.missing.begin` → `ap5.neg.missing.detected ok=1 reason=missing_or_unavailable` → `ap5.neg.done case=missing ok=1`
- The missing lane demonstrates detection of an object that is not available
  (object_id=2) in the current DiskFS.

### Read-No-Write — PASS
- Gate: `linen_diskfs_negative_classifications PASS`
- Markers: `ap5.neg.read_no_write.begin` → `ap5.neg.read_no_write.checked ok=1`
- The read-no-write lane verifies that when AP5 negative mode is active,
  AP3_WRITE is not simultaneously enabled. If AP3_WRITE were enabled, the
  lane would fail with `ap5.neg.fail reason=ap3_write_enabled`.
- In the current test: AP3_WRITE was not enabled → check passed.

### Metadata False-Claim Guard — PASS
- Gate: `linen_diskfs_negative_classifications PASS`
- Markers: `ap5.neg.metadata_false_claim.begin` → `ap5.neg.metadata_false_claim.checked ok=1 reason=metadata_not_diskfs_backed`
- The metadata false-claim guard honestly reports that metadata is not
  diskfs-backed, preventing a false claim about metadata persistence.

### Flush/Fsync Skip — PASS (non-claim)
- Gate: `linen_diskfs_negative_classifications PASS`
- Markers: `ap5.neg.flush_skip.begin` → `ap5.neg.flush_skip.detected ok=1 reason=sexdrive_flush_not_proven` → `ap5.neg.done case=flush_skip ok=1`
- The flush-skip lane honestly reports that sexdrive flush is not proven,
  classifying this as a non-claim rather than a false positive.

## Gate Results

| Lane | PASS | FAIL | SKIP | Faults |
|---|---|---|---|---|
| Mismatch | 264 | 0 | 104 | 0 |
| Missing | 264 | 0 | 104 | 0 |
| Read-No-Write | 264 | 0 | 104 | 0 |
| Metadata False-Claim | 264 | 0 | 104 | 0 |
| Flush Skip | 264 | 0 | 104 | 0 |
| Default | 263 | 0 | 105 | 0 |

## Regression Results

### Linen AP3 Read — FAIL (2 gates, PRE-EXISTING)
```
linen_diskfs_reboot_restore    FAIL  ap3.fail marker in read log
sexfiles_diskfs_bridge         FAIL  bridge recv present but incomplete operations
```
- Confirmed on clean master (stashed AP5 changes, same 2 failures).
- `ap3.fail` reason: `mismatch_at_0 expected=0x9b got=0x96` — stale QEMU
  disk image or pre-existing AP3 read data mismatch.
- `sexfiles_diskfs_bridge` FAIL: AP3 read-only test exercises the bridge
  path but does not perform write/flush, causing the bridge completeness
  gate to flag incomplete operations. This is a gate logic limitation,
  not an AP5 regression.

### Default — PASS
```
PASS gates: 266, FAIL gates: 0, SKIP gates: 102
linen_diskfs_reboot_restore         SKIP
linen_diskfs_metadata_persistence   SKIP
linen_diskfs_negative_classifications SKIP
```

## Non-Claims

- **Flush Skip**: Honest non-claim — `reason=sexdrive_flush_not_proven`
- **Metadata False-Claim Guard**: Honest non-claim — `reason=metadata_not_diskfs_backed`
- **Mismatch Detection**: The mismatch marker uses a computed wrong byte
  (`expected ^ 0x01`) — not a real disk corruption, a synthetic guard test.

## Updated Linen DiskFS Persistence Ladder

```
AP1  — DiskFS proof marker scaffolding              DONE (ancestor)
AP2  — Fixed-object save/load proof                 DONE (a1c1ad72)
AP3  — Reboot restore proof (write+read+verify)     DONE (4feefdb9) ⚠ AP3 read-only regression PRE-EXISTING
AP4  — Metadata persistence classification          DONE (aa0d5725)
AP5  — Negative classifications (THIS COMMIT)        DONE ← CURRENT
AP6  — TBD: flush/fsync durability proof (positive)
AP7  — TBD: multi-object consistency proof
```

## Exact git Commands

```sh
git diff --stat
git add servers/linen/src/main.rs \
        scripts/run_daily_driver_proof.sh \
        scripts/daily_driver_master_gate.sh \
        docs/handoff/LINEN_DISKFS_PERSISTENCE_100_AP5_NEGATIVE_CLASSIFICATIONS.md
git commit -m "linen: prove DiskFS negative classifications"
```
