# LINEN_DISKFS_PERSISTENCE_100_AP3_REBOOT_RESTORE

## 1. Files Changed

- `servers/linen/src/main.rs` — AP3_WRITE and AP3_READ consts, dispatch, proof functions
- `servers/linen/build.rs` — NEW: cargo rerun-if-env-changed for DiskFS proof envs
- `scripts/run_daily_driver_proof.sh` — AP3 env handling, image preservation logic
- `scripts/daily_driver_master_gate.sh` — `linen_diskfs_reboot_restore` gate

## 2. Exact Env Vars

- `SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP3_WRITE=1` — write boot
- `SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP3_READ=1` — read boot

Mutually exclusive: only one set per boot.

## 3. Write/Read Commands

### Write boot:
```
DAILY_DRIVER_PROBE_SECONDS=180 SEXOS_STORAGE_100_PROOF=1 \
  SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP3_WRITE=1 \
  ./scripts/run_daily_driver_proof.sh
```

### Read boot (same image):
```
DAILY_DRIVER_PROBE_SECONDS=180 SEXOS_STORAGE_100_PROOF=1 \
  SEXOS_LINEN_DISKFS_PERSISTENCE_100_AP3_READ=1 \
  ./scripts/run_daily_driver_proof.sh
```

## 4. Same nvme.img Preserved

Write boot creates/writes to `.gate_master/nvme.img`.
Read boot requires the image exists and refuses to recreate it.
Runner prints: `[proof] AP3 read boot: preserving existing nvme.img (no recreation)`

Image hash same before and after read boot: `7ef377b18f7b2d3b2436ab71cdbb36bb`

## 5. Object Identity

- object_id=1
- path_id=1 (`/disk/linen-object-v1`)
- Route: Linen → SLOT_STORAGE → SexFiles → DiskFS → SLOT_BLOCK → SexDrive → NVMe

## 6. Pattern Formula

```
byte[i] = (0xB6 ^ (i as u8) ^ 0x2D) & 0xFF
        = (0x9B ^ (i as u8)) & 0xFF
```

128 bytes, 8 chunks × 16 bytes for write, 16 chunks × 8 bytes for read.

## 7. Write Boot Result

```
[linen.diskfs100.ap3.write.begin] object_id=1 bytes=128
[linen.diskfs100.ap3.metadata.skip] reason=metadata_not_diskfs_backed
[linen.diskfs100.ap3.write.select.ok] path_id=1
[linen.diskfs100.ap3.write.stat.ok] size=4096 flags=0x3
[linen.diskfs100.ap3.write.chunk] off=0,16,32,48,64,80,96,112 len=16 ok=1
[linen.diskfs100.ap3.write.done] bytes=128 ok=1
[linen.diskfs100.ap3.flush.ok]
[linen.diskfs100.ap3.write.readback.request] object_id=1 size=128
[linen.diskfs100.ap3.write.readback.chunk] off=0..120 len=8 ok=1
[linen.diskfs100.ap3.write.readback] read=128
[linen.diskfs100.ap3.write.readback.match] bytes=128 ok=1
[linen.diskfs100.ap3.write.all_done] ok=1
```

Gate: PASS `AP3 write boot: chunks written + readback match + all_done ok=1`

## 8. Read Boot Result

```
[linen.diskfs100.ap3.read.begin] object_id=1 bytes=128
[linen.diskfs100.ap3.metadata.skip] reason=metadata_not_diskfs_backed
[linen.diskfs100.ap3.read.select.ok] path_id=1
[linen.diskfs100.ap3.read.stat.ok] size=4096 flags=0x3
[linen.diskfs100.ap3.read.chunk] off=0..120 len=8 ok=1
[linen.diskfs100.ap3.read.read] read=128
[linen.diskfs100.ap3.read.match] bytes=128 ok=1
[linen.diskfs100.ap3.read.done] ok=1
```

Gate: PASS `AP3 read boot: chunks read + byte match + done ok=1`

## 9. No-Write-in-Read-Mode Proof

The read boot log contains NO `linen.diskfs100.ap3.write.*` markers.
The gate explicitly checks for write markers in read logs and would FAIL if found.
The read proof function body contains no `OP_DISKFS_WRITE` or `OP_DISKFS_FLUSH` calls.

## 10. Gate Results

- Write boot: `linen_diskfs_reboot_restore PASS  AP3 write boot: chunks written + readback match + all_done ok=1`
- Read boot: `linen_diskfs_reboot_restore PASS  AP3 read boot: chunks read + byte match + done ok=1`
- Default boot: `linen_diskfs_reboot_restore SKIP  AP3 reboot restore proof not triggered`
- Faults: 0 in both logs
- cqe_timeout: 0 in both logs

## 11. AP2 Regression

AP2 fixed-object save/load:
```
linen_diskfs_fixed_object_save_load PASS  content match ok=1 bytes=128
```
No regression.

## 12. DiskFS AP4 Regression

SexFiles DiskFS AP4 reboot persistence:
```
sexfiles_diskfs_bridge_reboot_persistence PASS  AP4 read boot: chunks read + byte match + done ok=1
```
No regression (requires fresh AP4 write before read).

## 13. Default Boot Result

Both gates correctly SKIP in default mode:
```
linen_diskfs_fixed_object_save_load SKIP  AP2 fixed-object save/load proof not triggered
linen_diskfs_reboot_restore  SKIP      AP3 reboot restore proof not triggered
```

## 14. Non-Claims

- NO metadata DiskFS persistence — metadata is RamFS-only, honestly skipped
- NO Quil integration
- NO folders/path semantics beyond fixed path_id=1
- NO POSIX
- NO flush/power-loss durability
- NO crash consistency
- NO journaling
- NO kernel edits
- NO sex-pdx ABI edits
- NO apps/sexdrive edits
- NO servers/sexfiles edits
- NO broad refactor
