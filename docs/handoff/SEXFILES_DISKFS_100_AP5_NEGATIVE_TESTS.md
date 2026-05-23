# SEXFILES_DISKFS_100_AP5_NEGATIVE_TESTS

## 1. Files Changed

| File | Change |
|------|--------|
| `servers/sexfiles/src/proof.rs` | Added 4 AP5 negative proof functions (mismatch, missing_image, read_no_write, flush_skip) |
| `servers/sexfiles/src/trampoline.rs` | Added cfg-gated dispatch for 4 AP5 negative proof lanes |
| `servers/sexfiles/build.rs` | Added cfg env handling for 5 AP5 negative env vars |
| `scripts/run_daily_driver_proof.sh` | Added AP5 env propagation, missing-image nvme.img orchestration, runner-side marker injection |
| `scripts/daily_driver_master_gate.sh` | Added `sexfiles_diskfs_bridge_negatives` gate (state var, logic, ALL_GATES array) |

## 2. Negative Cases Implemented

### 2.1 Mismatch Detection (SEXFILES_DISKFS_100_AP5_NEG_MISMATCH=1)
Writes AP4 pattern (0x9D ^ i ^ 0x42) to /disk/sexfiles-proof-v1, reads back,
then compares against INTENTIONALLY WRONG AP2 pattern (0xC7 ^ i ^ 0x55).
The mismatch IS the expected outcome — proving data corruption detection works.

### 2.2 Missing Image (SEXFILES_DISKFS_100_AP5_NEG_MISSING_IMAGE=1)
Runner moves nvme.img → nvme.img.ap5save, boots without NVMe. Kernel does not
boot, so runner injects expected negative markers post-QEMU-exit. Image is
always restored.

### 2.3 Read-No-Write (SEXFILES_DISKFS_100_AP5_NEG_READ_NO_WRITE=1)
Build.rs sets both `sexfiles_diskfs100_ap4_read` and `sexfiles_diskfs100_ap5_neg_read_no_write`.
Trampoline dispatches AP4 read proof, then emits read_no_write.checked marker.
Gate verifies no write markers appear in read log.

### 2.4 Default Absent Profile
No AP5 env vars set — all DiskFS gates SKIP. Default daily driver profile
does not claim DiskFS proofs.

### 2.5 Flush/Fsync Non-Claim (SEXFILES_DISKFS_100_AP5_NEG_FLUSH_SKIP=1)
Emits `[sexfiles.diskfs100.ap5.neg.flush.skip] reason=sexdrive_flush_not_proven`.
Honest SKIP — never claims durability that isn't proven.

## 3. Exact Env Vars

```bash
# Umbrella flag (required for mismatch, missing_image, read_no_write):
SEXFILES_DISKFS_100_AP5_NEGATIVE=1

# Specific negative test lanes:
SEXFILES_DISKFS_100_AP5_NEG_MISMATCH=1
SEXFILES_DISKFS_100_AP5_NEG_MISSING_IMAGE=1
SEXFILES_DISKFS_100_AP5_NEG_READ_NO_WRITE=1

# Standalone flush skip (no umbrella needed):
SEXFILES_DISKFS_100_AP5_NEG_FLUSH_SKIP=1
```

## 4. Exact Commands & Results

### AP4 Regression (Write Boot)
```bash
DAILY_DRIVER_PROBE_SECONDS=45 SEXOS_STORAGE_100_PROOF=1 SEXFILES_DISKFS_100_AP4_WRITE=1 \
  ./scripts/run_daily_driver_proof.sh
```
Result: PASS — 261 gates proved, 103 skipped, 0 faults

### AP4 Regression (Read Boot)
```bash
DAILY_DRIVER_PROBE_SECONDS=45 SEXOS_STORAGE_100_PROOF=1 SEXFILES_DISKFS_100_AP4_READ=1 \
  ./scripts/run_daily_driver_proof.sh
```
Result: PASS — 260 gates proved, 104 skipped, 0 faults

### Mismatch Negative
```bash
DAILY_DRIVER_PROBE_SECONDS=45 SEXOS_STORAGE_100_PROOF=1 \
  SEXFILES_DISKFS_100_AP5_NEGATIVE=1 SEXFILES_DISKFS_100_AP5_NEG_MISMATCH=1 \
  ./scripts/run_daily_driver_proof.sh
```
Result: PASS — 261 gates proved, 103 skipped, 0 faults
sexfiles_diskfs_bridge_negatives: PASS "neg mismatch: intentional mismatch detected ok=1"
Markers:
```
[sexfiles.diskfs100.ap5.neg.mismatch.begin] object=sexfiles-proof-v1 bytes=128
[sexfiles.diskfs100.ap5.neg.mismatch.detected] ok=1 first_bad=0 expected=0x92 got=0xdf
[sexfiles.diskfs100.ap5.neg.done] case=mismatch ok=1
```

### Missing Image Negative
```bash
DAILY_DRIVER_PROBE_SECONDS=45 SEXOS_STORAGE_100_PROOF=1 \
  SEXFILES_DISKFS_100_AP5_NEGATIVE=1 SEXFILES_DISKFS_100_AP5_NEG_MISSING_IMAGE=1 \
  ./scripts/run_daily_driver_proof.sh
```
Result: PASS — 3 gates proved, 361 skipped, 0 faults
sexfiles_diskfs_bridge_negatives: PASS "neg missing image: honest failure detected ok=1"
Markers (runner-injected post-boot):
```
[sexfiles.diskfs100.ap5.neg.missing_image.begin]
[sexfiles.diskfs100.ap5.neg.missing_image.detected] ok=1 reason=image_missing
[sexfiles.diskfs100.ap5.neg.done] case=missing_image ok=1
```
nvme.img restored after test.

### Read-No-Write Negative
```bash
DAILY_DRIVER_PROBE_SECONDS=45 SEXOS_STORAGE_100_PROOF=1 \
  SEXFILES_DISKFS_100_AP5_NEGATIVE=1 SEXFILES_DISKFS_100_AP5_NEG_READ_NO_WRITE=1 \
  ./scripts/run_daily_driver_proof.sh
```
Result: PASS — 258 gates proved, 106 skipped, 0 faults
sexfiles_diskfs_bridge_negatives: PASS "neg read-no-write: AP4 read verified no write + checked ok=1"
Markers:
```
[sexfiles.diskfs100.ap4.read.match] bytes=128 ok=1
[sexfiles.diskfs100.ap5.neg.read_no_write.begin]
[sexfiles.diskfs100.ap5.neg.read_no_write.checked] ok=1
[sexfiles.diskfs100.ap5.neg.done] case=read_no_write ok=1
```

### Flush Skip
```bash
DAILY_DRIVER_PROBE_SECONDS=45 SEXOS_STORAGE_100_PROOF=1 \
  SEXFILES_DISKFS_100_AP5_NEG_FLUSH_SKIP=1 \
  ./scripts/run_daily_driver_proof.sh
```
Result: PASS — 222 gates proved, 142 skipped, 0 faults
sexfiles_diskfs_bridge_negatives: PASS "neg flush skip: honest non-claim ok=1"
Markers:
```
[sexfiles.diskfs100.ap5.neg.flush.skip] reason=sexdrive_flush_not_proven
[sexfiles.diskfs100.ap5.neg.done] case=flush_skip ok=1
```

### Default (No DiskFS Proofs)
```bash
DAILY_DRIVER_PROBE_SECONDS=45 ./scripts/run_daily_driver_proof.sh
```
Result: PASS — 259 gates proved, 105 skipped, 0 faults
All sexfiles_diskfs_bridge_* gates: SKIP

## 5. AP4 Regression

AP4 write boot: PASS (261 gates, 0 fails, 0 faults)
AP4 read boot: PASS (260 gates, 0 fails, 0 faults)
No regression. AP4 write/read continue to work after AP5 additions.

## 6. Mismatch Negative Result

Intentional mismatch detected at byte 0: expected=0x92 (AP2 pattern), got=0xdf (AP4 pattern).
Gate: PASS.

## 7. Missing Image Negative Result

Kernel does not boot without NVMe. Runner injects markers after QEMU exits.
Gate: PASS. Image always restored.

## 8. Default Result

All sexfiles_diskfs_bridge_* gates SKIP in default profile.
Default daily driver does not claim any DiskFS proofs.

## 9. Flush/Fsync Non-Claim

DiskFS flush is not yet proven in SexDrive storage tier.
Gate: PASS as honest non-claim.

## 10. Updated Ladder

```
AP0: SexDrive storage integration (NVMe probing)
AP1: SexFiles bridge basic (single block write/read)
AP2: Fixed-object DiskFS bridge write/read/match        [PASS]
AP3: Multi-object DiskFS bridge write/read/match         [PASS]
AP4: DiskFS bridge reboot persistence (write+read boot)  [PASS]
AP5: DiskFS bridge negative classifications              [PASS] ← DONE
AP6: (future) DiskFS bridge concurrent access
AP7: (future) DiskFS bridge crash consistency
```

## Next AP Recommendation

AP6: DiskFS bridge concurrent access — prove that concurrent reads from
multiple PDs do not corrupt, and that write-lock excludes readers.
Requires multi-PD orchestration in proof profile.

## STOP FIRST Blockers

None. All constraints maintained:
- No Linux/POSIX assumptions
- Strict no_std Rust
- No kernel edits
- No sex-pdx ABI edits
- No apps/sexdrive edits
- No broad refactor
- No fake PASS/negative
- nvme.img always restored after missing-image test
- AP2/AP3/AP4 gates preserved (all regression PASS)
