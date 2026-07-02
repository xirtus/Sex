# SEXFILES_PERSISTENCE_REBOOT_PROOF_V1

## Goal
Prove write persistence across reboot using reserved NVMe proof LBA:
- Boot A writes marker to LBA 2047 and verifies immediate readback.
- Boot B reads LBA 2047 before any write and verifies marker match.

## What Changed

### scripts/master_runtime_gate.sh
- Added two-boot persistence mode controlled by:
  - `SEXOS_PERSISTENCE_REBOOT_PROOF=1`
- Added optional image preservation mode:
  - `SEXOS_NVME_PRESERVE_IMG=1`
- Added split serial logs:
  - `.gate_master/serial.boot_a.log`
  - `.gate_master/serial.boot_b.log`
- In persistence mode:
  - Runs Boot A then Boot B against the same `nvme.img`.
  - Avoids recreating `nvme.img` between A and B.
  - Prints grouped marker summaries for both boots.

### servers/sexfiles/src/proof.rs
- Added persistence sequencing markers and logic:
  - Boot B read-before-write probe markers:
    - `[sexfiles.persistence.boot_b.begin]`
    - `[sexfiles.persistence.boot_b.read_before_write.begin]`
  - If marker matches expected `(magic,lba,tag)`:
    - `[sexfiles.persistence.boot_b.read_before_write.match] ...`
  - If marker does not match:
    - `[sexfiles.persistence.boot_b.read_before_write.mismatch] ...`
    - fallback to Boot A flow:
      - `[sexfiles.persistence.boot_a.begin]`
      - `[sexfiles.persistence.boot_a.write.ok]`
      - `[sexfiles.persistence.boot_a.readback.match]`

## How Persistence Was Preserved
- Boot A and Boot B run inside one gate invocation in persistence mode.
- Both boots use the same `.gate_master/nvme.img`.
- No image recreation/zeroing occurs between Boot A and Boot B.
- For clean proof runs, remove stale image once before running:
  - `rm -f .gate_master/nvme.img`

## Proof Run

### Command
```bash
rm -f .gate_master/nvme.img
SEXOS_GATE_NVME=1 SEXOS_SEXFILES_REAL_BLOCK_PROOF=1 SEXOS_PERSISTENCE_REBOOT_PROOF=1 \
./scripts/master_runtime_gate.sh --probe 25 --keep-log
```

### Expected/Observed Boot A Markers
- `[sexfiles.persistence.boot_b.read_before_write.mismatch] ...` (clean image baseline)
- `[sexfiles.persistence.boot_a.begin]`
- `[sexfiles.persistence.boot_a.write.ok]`
- `[sexfiles.persistence.boot_a.readback.match]`

### Expected/Observed Boot B Markers
- `[sexfiles.persistence.boot_b.begin]`
- `[sexfiles.persistence.boot_b.read_before_write.begin]`
- `[sexfiles.persistence.boot_b.read_before_write.match] magic=0x3156455449525753 lba=2047 tag=0xa5a5a5a5a5a5a5a5`

## Safety/Negative Expectations
- No `#PF/#GP/panic` in Boot A or Boot B serial logs.
- LBA0 write remains denied by guard.
- No fake persistence path.

## Final Grep Commands
```bash
grep -E 'sexfiles\.persistence\.boot_a|sexfiles\.persistence\.boot_b|sexdrive\.nvme\.write|#PF|#GP|panic' \
  .gate_master/serial.boot_a.log .gate_master/serial.boot_b.log

grep -E 'sexfiles\.persistence\.boot_b\.read_before_write\.(begin|match|mismatch)' \
  .gate_master/serial.boot_b.log
```

## Next Prompt
- `SEXFILES_STORAGE_FAULT_NEGATIVE_V1`
- If negative coverage is already complete in your lane: `FINAL_SEXFILES_SEXDRIVE_AUDIT_V1`
