# SexFiles DiskFS 100 AP4 Reboot Persistence Proof

**Status:** PROVEN (2026-05-23)

## 1. Files Changed

| File | Change |
|------|--------|
| `servers/sexfiles/src/proof.rs` | +225 lines: `run_diskfs100_ap4_write_proof()`, `run_diskfs100_ap4_read_proof()` |
| `servers/sexfiles/src/trampoline.rs` | +16 lines: AP4 write/read dispatch before AP2/AP3 |
| `servers/sexfiles/build.rs` | +10 lines: cfg flags `sexfiles_diskfs100_ap4_write`, `sexfiles_diskfs100_ap4_read` |
| `scripts/run_daily_driver_proof.sh` | +10 lines: AP4 env vars, image preservation logic |
| `scripts/daily_driver_master_gate.sh` | +74 lines: gate `sexfiles_diskfs_bridge_reboot_persistence` |

Total: 5 files, +335 lines.

## 2. Exact Environment Variables

**Write boot:**
```
SEXOS_STORAGE_100_PROOF=1
SEXFILES_DISKFS_100_AP4_WRITE=1
DAILY_DRIVER_PROBE_SECONDS=180
```

**Read boot:**
```
SEXOS_STORAGE_100_PROOF=1
SEXFILES_DISKFS_100_AP4_READ=1
DAILY_DRIVER_PROBE_SECONDS=180
```

## 3. Exact Write/Read Commands

```bash
# Write boot
rm -f .gate_master/nvme.img  # optional: start clean
DAILY_DRIVER_PROBE_SECONDS=180 SEXOS_STORAGE_100_PROOF=1 SEXFILES_DISKFS_100_AP4_WRITE=1 ./scripts/run_daily_driver_proof.sh
cp /tmp/sexos_daily_driver_proof.log /tmp/sexfiles_diskfs_ap4_write.log

# Read boot (same image, no deletion)
DAILY_DRIVER_PROBE_SECONDS=180 SEXOS_STORAGE_100_PROOF=1 SEXFILES_DISKFS_100_AP4_READ=1 ./scripts/run_daily_driver_proof.sh
cp /tmp/sexos_daily_driver_proof.log /tmp/sexfiles_diskfs_ap4_read.log
```

## 4. Confirmation: Same nvme.img Preserved

```
nvme.img before read boot: c59d688d3c35328c82923b59d7142256
nvme.img after read boot:  c59d688d3c35328c82923b59d7142256
→ SHA unchanged (image not recreated between boots)
```

Runner explicitly prints: `[proof] AP4 read boot: preserving existing nvme.img (no recreation)` and exits fatally if the image is missing in read mode.

## 5. Object Identity

- Object: `/disk/sexfiles-proof-v1` (path_id=0)
- Manifest: V2 manifest via `diskfs_ensure_manifest_v2`
- SELECT: `diskfs_lookup_by_path_id(0, buf_va)`
- Buffer: `diskfs_bridge_get_buf_va()` grant

## 6. Pattern Formula

```
byte[i] = (0x9D ^ i ^ 0x42) & 0xFF  for i in 0..128
```

This pattern is distinct from AP2 (`0xC7 ^ i ^ 0x55`) to prevent false positives from stale data.

## 7. Write Boot Result

```
Gate: sexfiles_diskfs_bridge_reboot_persistence → PASS
Markers:
  [sexfiles.diskfs100.ap4.write.begin] object=sexfiles-proof-v1 bytes=128
  [sexfiles.diskfs100.ap4.write.select.ok] object=sexfiles-proof-v1
  [sexfiles.diskfs100.ap4.write.chunk] off=0/16/32/48/64/80/96/112 len=16 ok=1 (8 chunks)
  [sexfiles.diskfs100.ap4.write.readback.chunk] off=0/16/32/48/64/80/96/112 len=16 ok=1 (8 chunks)
  [sexfiles.diskfs100.ap4.write.match] bytes=128 ok=1
  [sexfiles.diskfs100.ap4.write.done] bytes=128 ok=1
```

## 8. Read Boot Result

```
Gate: sexfiles_diskfs_bridge_reboot_persistence → PASS
Markers:
  [sexfiles.diskfs100.ap4.read.begin] object=sexfiles-proof-v1 bytes=128
  [sexfiles.diskfs100.ap4.read.select.ok] object=sexfiles-proof-v1
  [sexfiles.diskfs100.ap4.read.chunk] off=0/16/32/48/64/80/96/112 len=16 ok=1 (8 chunks)
  [sexfiles.diskfs100.ap4.read.match] bytes=128 ok=1
  [sexfiles.diskfs100.ap4.read.done] ok=1
```

Confirm: NO `write.begin` or `write.chunk` or `write.done` markers in read log.
Data was read from the same NVMe image, byte-matched the expected AP4 pattern.

## 9. Gate Result

- Write log: `sexfiles_diskfs_bridge_reboot_persistence` → PASS (write.boot profile)
- Read log: `sexfiles_diskfs_bridge_reboot_persistence` → PASS (read.boot profile)
- Gate semantics: full AP4 acceptance requires BOTH logs PASS with same preserved nvme.img

## 10. Non-Claims

- **No flush/power-loss durability claim.**
- **No Linen involvement** (direct DiskFS bridge, same object as AP2).
- **No generic filesystem semantics** beyond the proven DiskFS bridge object.
- **No POSIX/Linux semantics.**
- The read boot mounts the same image; journal replay may apply but the data is proven readable.
- The runner does NOT simulate a crash or power loss between boots.
- No cross-reboot atomicity/ordering guarantee beyond what is directly observed.

## 11. Regression Results

| Proof | Result |
|-------|--------|
| AP3 (`SEXFILES_DISKFS_100_AP3_PROOF=1`) | PASS (266 gates, 0 FAIL, 0 faults) |
| Default (`./scripts/run_daily_driver_proof.sh`) | PASS (257 gates, 0 FAIL, 0 faults) |
| AP4 write boot | PASS (261 gates, 0 FAIL, 0 faults) |
| AP4 read boot | PASS (260 gates, 0 FAIL, 0 faults) |

## 12. Updated Ladder

```
AP1: SexDrive storage IOQ ready                        PASS
AP2: DiskFS fixed-object bridge RW/match               PASS
AP3: DiskFS multi-object bridge RW/match               PASS
AP4: DiskFS reboot persistence (two-boot)              PASS ← this proof
AP5: Storage flush durability / crash safety           FUTURE
AP6: Negative tests / fault injection                  FUTURE
```

## 13. Next AP Recommendation

AP5: Storage flush durability — prove that data survives a simulated crash/power-loss by forcing NVMe flush, then verifying data integrity on remount. Requires accurate CQE tracking and flush-commit semantics.
