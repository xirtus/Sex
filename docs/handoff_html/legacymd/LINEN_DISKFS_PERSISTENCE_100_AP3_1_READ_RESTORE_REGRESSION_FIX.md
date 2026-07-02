# LINEN_DISKFS_PERSISTENCE_100 AP3 READ RESTORE REGRESSION FIX

Date: 2026-05-25
Branch: master (e7978616 + fix)

## 1. Files Changed

- `scripts/daily_driver_master_gate.sh` — gate scanner conditional logic fix

## 2. Reproduction Evidence

AP3 WRITE boot: PASS — `linen_diskfs_reboot_restore PASS`, 0 FAIL gates
AP3 READ boot (pre-fix): `linen_diskfs_reboot_restore PASS`, but `sexfiles_diskfs_bridge FAIL` causing FINAL: FAIL

### Pre-fix AP3 READ gate output:
```
linen_diskfs_reboot_restore  PASS   AP3 read boot: chunks read + byte match + done ok=1
sexfiles_diskfs_bridge       FAIL   bridge recv present but incomplete operations
FINAL: FAIL (1 gate(s) failed)
```

### QEMU log confirmation (AP3 read passing at application level):
```
linen.diskfs100.ap3.read.match bytes=128 ok=1
linen.diskfs100.ap3.read.done ok=1
```
All 16 chunks: ok=1. Zero panics. Zero cqe_timeout.

## 3. Root Cause Answers (PHASE C)

1. **Did AP3 WRITE boot write the expected AP3 pattern?** YES — 8 chunks of 16 bytes written + readback verified.
2. **Did AP3 READ boot use the exact same pattern formula?** YES — 16 chunks of 8 bytes read back, all matched.
3. **Did AP3 READ boot write before reading?** NO — read-only boot correctly preserved nvme.img.
4. **Was .gate_master/nvme.img preserved or recreated between commands?** PRESERVED — runner correctly detected existing image and skipped recreation.
5. **Did runner accidentally enable AP2/AP5/AP4 lanes during AP3 READ?** NO — profile was clean: only ap3_read=1, all others zero.
6. **Is AP3 READ reading correct path_id/object path?** YES — `path_id=1`, `object_id=1`.
7. **Is mismatch got byte equal to AP2/AP4/AP5 pattern?** N/A — no application-level mismatch occurred. All bytes matched.
8. **Are DiskFS/SexDrive block operations status=0 with no cqe_timeout?** YES — all block reads had status=0, no cqe_timeout markers.
9. **Did AP3 read mode compile with wrong option_env because env export is sticky?** NO — compilation was clean.
10. **Is the failure due to gate parsing or actual AP3 fail marker?** GATE PARSING failure. The `sexfiles_diskfs_bridge` gate unconditionally required `write.ok` and `flush.ok` success markers, but AP3 READ only exercises READ operations through the bridge (no WRITE, no FLUSH). The gate already had conditional logic for `stat` (op=0x3B) and `manifest` (op=0x3C) — requiring success only when those ops were recv'd. WRITE (op=0x38) and FLUSH (op=0x3A) lacked equivalent conditional guards.

## 4. Minimal Fix

In `scripts/daily_driver_master_gate.sh`, gate `sexfiles_diskfs_bridge`:

Added `need_write` and `need_flush` conditional checks mirroring the existing `need_stat` and `need_manifest` pattern:

```bash
has_write_recv=$(has 'sexfiles\.bridge\.diskfs\.recv.*op=0x38')
has_flush_recv=$(has 'sexfiles\.bridge\.diskfs\.recv.*op=0x3A')
need_write=1; [ "$has_write_recv" -eq 1 ] || need_write=0
need_flush=1; [ "$has_flush_recv" -eq 1 ] || need_flush=0
```

Added effective variables:
```bash
write_ok_effective=1
if [ "$need_write" -eq 1 ] && [ "$has_write_ok" -eq 0 ]; then write_ok_effective=0; fi
flush_ok_effective=1
if [ "$need_flush" -eq 1 ] && [ "$has_flush_ok" -eq 0 ]; then flush_ok_effective=0; fi
```

Changed success marker check to use effective variables:
- `[ "$write_ok_effective" -eq 1 ]` (was: `[ "$has_write_ok" -eq 1 ]`)
- `[ "$flush_ok_effective" -eq 1 ]` (was: `[ "$has_flush_ok" -eq 1 ]`)

## 5. Fixed AP3 Write/Read Results

### AP3 WRITE:
```
linen_diskfs_reboot_restore  PASS   AP3 write boot: chunks written + readback match + all_done ok=1
sexfiles_diskfs_bridge       PASS   bridge op success markers complete
faults_zero                  PASS   0 fault markers
FAIL gates: 0
FINAL: PASS (268 gates proved, 100 skipped, 0 faults)
```

### AP3 READ:
```
linen_diskfs_reboot_restore  PASS   AP3 read boot: chunks read + byte match + done ok=1
sexfiles_diskfs_bridge       PASS   bridge op success markers complete
faults_zero                  PASS   0 fault markers
FAIL gates: 0
FINAL: PASS (268 gates proved, 100 skipped, 0 faults)
```

## 6. Regression Results

- **AP2**: PASS — `linen_diskfs_fixed_object_save_load PASS`, 0 FAIL gates, 0 faults
- **AP4**: PASS — `linen_diskfs_metadata_persistence PASS`, 0 FAIL gates, 0 faults
- **AP5**: PASS — `linen_diskfs_negative_classifications PASS`, 0 FAIL gates, 0 faults
- **Default**: PASS — 0 FAIL gates, 0 faults

## 7. Non-Claims

- No metadata DiskFS persistence claimed
- No Quil involvement
- No folders/path/POSIX semantics
- No flush/power-loss durability
- No crash consistency
- No kernel edits
- No sex-pdx ABI edits
- No apps/sexdrive edits
- No servers/sexfiles edits
- No Linen source changes
- No gate weakening (existing conditional pattern extended consistently)
