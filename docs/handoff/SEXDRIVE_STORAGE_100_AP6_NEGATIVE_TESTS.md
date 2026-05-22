# SEXDRIVE_STORAGE_100_AP6_NEGATIVE_TESTS

## 1) Files changed
- apps/sexdrive/src/main.rs
- scripts/run_daily_driver_proof.sh
- scripts/daily_driver_master_gate.sh

## 2) Negative cases implemented
- Mismatch negative: implemented in SexDrive with explicit markers:
  - [sexdrive.storage100.neg.mismatch.begin] lba=384 bytes=512
  - [sexdrive.storage100.neg.mismatch.detected] ok=1 ...
- Missing-image negative: implemented in runner/gate flow with safe image move/restore and explicit expected-fail markers:
  - [sexdrive.storage100.neg.missing_image.begin]
  - [sexdrive.storage100.neg.missing_image.fail_expected] ok=1 reason=image_missing
- Absent profile/default lane: storage gates remain SKIP by default.
- Flush unsupported (AP5b): remains SKIP in this environment when not completed/supported.

## 3) Exact env vars
- SEXOS_STORAGE_100_PROOF=1
- SEXOS_STORAGE_100_NEGATIVE=1
- SEXOS_STORAGE_100_NEG_MISMATCH=1
- SEXOS_STORAGE_100_NEG_MISSING_IMAGE=1
- Existing persistence/flush controls remain:
  - SEXOS_STORAGE_100_PERSIST_WRITE
  - SEXOS_STORAGE_100_PERSIST_READ
  - SEXOS_STORAGE_100_FLUSH_AUDIT

## 4) Exact commands used
- bash -n scripts/run_daily_driver_proof.sh
- bash -n scripts/daily_driver_master_gate.sh
- SEXOS_STORAGE_100_PROOF=1 ./scripts/run_daily_driver_proof.sh
- ./scripts/daily_driver_master_gate.sh /tmp/sexos_daily_driver_proof.log | grep -E "sexdrive_storage_ioq_ready|sexdrive_storage_single_block_rw|sexdrive_storage_multiblock_rw|sexdrive_storage_reboot_persistence|sexdrive_storage_flush_durability|sexdrive_storage_negatives|FAIL gates|FINAL"
- SEXOS_STORAGE_100_PROOF=1 SEXOS_STORAGE_100_NEGATIVE=1 SEXOS_STORAGE_100_NEG_MISMATCH=1 ./scripts/run_daily_driver_proof.sh
- cp /tmp/sexos_daily_driver_proof.log /tmp/sexos_storage_ap6_mismatch.log
- ./scripts/daily_driver_master_gate.sh /tmp/sexos_storage_ap6_mismatch.log | grep -E "sexdrive_storage_negatives|FAIL gates|FINAL"
- SEXOS_STORAGE_100_PROOF=1 SEXOS_STORAGE_100_NEGATIVE=1 SEXOS_STORAGE_100_NEG_MISSING_IMAGE=1 ./scripts/run_daily_driver_proof.sh
- cp /tmp/sexos_daily_driver_proof.log /tmp/sexos_storage_ap6_missing_image.log
- SEXOS_STORAGE_100_PROOF=0 ./scripts/daily_driver_master_gate.sh /tmp/sexos_storage_ap6_missing_image.log | grep -E "sexdrive_storage_negatives|FAIL gates|FINAL"
- ./scripts/run_daily_driver_proof.sh
- ./scripts/daily_driver_master_gate.sh /tmp/sexos_daily_driver_proof.log | grep -E "sexdrive_storage|FAIL gates|FINAL"

## 5) Positive storage regression result
- Observed in positive storage lane:
  - sexdrive_storage_ioq_ready PASS
  - sexdrive_storage_single_block_rw PASS
  - sexdrive_storage_multiblock_rw PASS
  - sexdrive_storage_reboot_persistence SKIP (not triggered in that log)
  - sexdrive_storage_flush_durability SKIP (flush not completed/supported)
  - sexdrive_storage_negatives SKIP
  - FINAL PASS

## 6) Mismatch negative result
- Evidence markers in `/tmp/sexos_storage_ap6_mismatch.log`:
  - [sexdrive.storage100.neg.mismatch.begin] lba=384 bytes=512
  - [sexdrive.storage100.neg.mismatch.detected] ok=1 first_bad=0 expected=152 got=153
- Gate result:
  - sexdrive_storage_negatives PASS
  - FAIL gates: 0
  - FINAL: PASS

## 7) Missing image negative result
- Safe move/restore path used (`nvme.img` -> `nvme.img.ap6save` -> restore).
- Evidence markers in `/tmp/sexos_storage_ap6_missing_image.log`:
  - [sexdrive.storage100.neg.missing_image.begin]
  - [sexdrive.storage100.neg.missing_image.fail_expected] ok=1 reason=image_missing
- Gate result:
  - sexdrive_storage_negatives PASS
  - FAIL gates: 0
  - FINAL: PASS

## 8) Flush unsupported classification
- AP5b remains SKIP when flush completion is not supported/completed in this environment.

## 9) Default boot result
- Default daily lane keeps storage gates SKIP and FINAL PASS.

## 10) Updated ladder
- AP2 PASS
- AP3 PASS
- AP4 PASS
- AP5a PASS (from prior proven ladder)
- AP5b SKIP honest
- AP6 negatives PASS
- AP7 closeout/tag pending
