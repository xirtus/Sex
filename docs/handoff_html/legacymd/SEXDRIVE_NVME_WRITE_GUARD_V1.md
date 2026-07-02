# SEXDRIVE_NVME_WRITE_GUARD_V1

## Mission
Define and enforce a safe reserved write-test range before any real NVMe WRITE implementation.

## Result
PASS (guard-only, no write execution)

## 1) Reserved write range decision
- Reserved proof write region is the **final LBA** of gate NVMe image:
  - `WRITE_PROOF_LBA = 2047`
  - `WRITE_PROOF_LEN = 512`
  - `WRITE_PROOF_OFFSET = 0xffe00`
- Rationale:
  - Gate image uses 2048 sectors (`bs=512 count=2048`)
  - Final-sector-only policy avoids LBA0 writes.
- Documented marker constant:
  - `WRITE_PROOF_MAGIC = 0x3156455449525753` (`"SWRITEV1"` LE tag for proof config visibility)

## 2) Guard rules
In `apps/sexdrive/src/main.rs` `BLOCK_WRITE` path:
- Proof mode condition: `buf_cap == SLOT_BUF_LEND`
- Allow only when all hold:
  - proof mode true
  - `offset == WRITE_PROOF_LBA * 512`
  - `size == 512`
- Emit markers for config + decision:
  - `[sexdrive.write.guard.config]`
  - `[sexdrive.write.guard.begin]`
  - `[sexdrive.write.guard.allow]` or `[sexdrive.write.guard.deny]`
- **No actual NVMe WRITE command issued** in this mission.
- Status remains honest:
  - `BLOCK_WRITE -> ERR_NO_DEVICE` (`write_not_implemented_guard_only`)

## 3) Denied cases
Observed in runtime:
- `offset=0 size=512 buf_cap=0` -> denied (`proof_mode=0`)
- `size=4096` remains denied by existing typed bad-len path (ERR_BAD_LEN)
- write without proof mode denied (guard marker + ERR_NO_DEVICE)

## 4) Files changed
- `apps/sexdrive/src/main.rs`
- `servers/sexfiles/src/proof.rs` (added a write-guard probe call/marker only)
- `docs/handoff/SEXDRIVE_NVME_WRITE_GUARD_V1.md`

## 5) Proof markers observed
From `.gate_master/serial.log`:
- `[sexdrive.write.guard.config] proof_lba=2047 proof_offset=0xffe00 proof_len=512 ...`
- `[sexdrive.write.guard.begin] offset=0x0 size=512 buf_cap=0x0 proof_mode=0`
- `[sexdrive.write.guard.deny] offset=0x0 size=512 buf_cap=0x0`
- `[sexdrive.write.guard.begin] offset=0xffe00 size=512 buf_cap=0x11 proof_mode=1`
- `[sexdrive.write.guard.allow] offset=0xffe00 size=512 buf_cap=0x11`
- `[sexfiles.block.proof.write_guard.probe] ... status=4 expected=ERR_NO_DEVICE(4)`

## Build/Runtime
- `build_payload.sh`: PASS
- Runtime gate build: PASS
- Read payload proof path still passes in same run
- Negative typed tests still pass (`typed_summary honest=1`)
- No `#PF/#GP/panic`
- `FINAL_SCORE` remains `RED_MASTER` only due unrelated `CLOCK_GATE` miss

## 6) Final grep command
```bash
grep -E "sexdrive\.write\.guard\.|sexfiles\.block\.proof\.write_guard\.probe|#PF|#GP|panic" .gate_master/serial.log
```

## 7) Next prompt
`SEXDRIVE_NVME_WRITE_READBACK_PROOF_V1`
