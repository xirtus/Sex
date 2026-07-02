# SEXFILES_STORAGE_FAULT_NEGATIVE_V1

## Goal
Prove the SexFiles -> SexDrive storage lane fails safely after persistence proof, without unsafe writes or fake success.

## Scope
- Negative/fault proofs only.
- No ABI or kernel changes.
- No new storage features.

## Changes
- Updated `servers/sexfiles/src/proof.rs` to add explicit negative markers and summary:
  - `sexfiles.storage.negative.begin`
  - `sexfiles.storage.negative.bad_cmd.ok`
  - `sexfiles.storage.negative.bad_len.ok`
  - `sexfiles.storage.negative.unaligned.ok`
  - `sexfiles.storage.negative.write_lba0_denied.ok`
  - `sexfiles.storage.negative.write_bad_cap.ok`
  - `sexfiles.storage.negative.write_bad_size.ok`
  - `sexfiles.storage.negative.memlend_no_cap.ok`
  - `sexfiles.storage.negative.summary honest=1`
  - `sexfiles.storage.negative.err` (error-only path)

## Negative Test Table
| Case | Expected | Observed |
|---|---|---|
| bad command | `ERR_BAD_CMD` | PASS (`status=1`) |
| read size 0 | `ERR_BAD_LEN` | PASS (`status=2`) |
| read size > BLOCK_MAX_XFER | `ERR_BAD_LEN` | PASS (`status=2`) |
| unaligned read offset | `ERR_BAD_LEN` | PASS (`status=2`) |
| write to LBA 0 | denied, no write submit | PASS (`status=4`, guard deny) |
| write wrong buf_cap | denied | PASS (`status=4`, guard deny) |
| write wrong size | denied | PASS (`status=4`, guard deny) |
| map empty MemLend slot | `u64::MAX` | PASS (`0xffffffffffffffff`) |
| map wrong cap kind | `u64::MAX` | PASS (`0xffffffffffffffff`) |

## Denied Write Proof
Observed deny markers in both boot logs:
- `[sexdrive.block.write.api.recv] ...`
- `[sexdrive.write.guard.deny] ...`

No NVMe write submit marker appeared in either boot log for these denied cases:
- No `[sexdrive.nvme.write.submit]`
- No `[sexdrive.nvme.write.cqe]`
- No `[sexdrive.nvme.write.ok]`

## Persistence After Negatives
Boot B read-before-write still matches persisted marker after negatives:
- `[sexfiles.persistence.boot_b.read_before_write.begin]`
- `[sexfiles.persistence.boot_b.read_before_write.match] magic=0x3156455449525753 lba=2047 tag=0xa5a5a5a5a5a5a5a5`

## Build/Gate
- `bash build_payload.sh`: PASS
- `SEXOS_GATE_NVME=1 SEXOS_SEXFILES_REAL_BLOCK_PROOF=1 SEXOS_PERSISTENCE_REBOOT_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log`
  - Typed/negative/persistence markers: PASS
  - Global gate remained RED due unrelated `CLOCK_GATE` failure.

## Final Grep Commands
```bash
grep -E 'sexfiles\.storage\.negative|sexfiles\.persistence\.boot_b\.read_before_write\.(begin|match|mismatch)|#PF|#GP|panic' \
  .gate_master/serial.boot_a.log .gate_master/serial.boot_b.log

grep -E 'sexdrive\.block\.write\.api\.recv|sexdrive\.write\.guard\.(allow|deny)|sexdrive\.nvme\.write\.(submit|cqe|ok|err)' \
  .gate_master/serial.boot_a.log .gate_master/serial.boot_b.log
```

## Next Prompt
- `FINAL_SEXFILES_SEXDRIVE_AUDIT_V1`
