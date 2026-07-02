# SEXDRIVE_NVME_IO_READ_ONE_BLOCK_PROOF_V1

- date: 2026-05-07
- scope: one direct NVMe READ proof on IO queue 1 only
- files touched: `apps/sexdrive/src/main.rs`, this handoff
- no BLOCK_READ API success wiring in this mission

## Result

PASS. One real NVMe READ command completed successfully on IO queue 1.

## Data Buffer Allocation

- `data_phys=0x102ae000`
- `data_va=0x40000000a000`
- page-aligned, mapped, zeroed before submission

## READ Command Fields

- opcode `0x02` (READ)
- CID `69`
- NSID `1`
- SLBA `0`
- NLB `0` (one logical block, zero-based)
- PRP1 = `0x102ae000`

Queue state used:
- qid `1`
- depth `16`
- initial `sq_tail=0`, `cq_head=0`, `cq_phase=1`

Doorbell:
- `SQ1TDBL=0x1008`
- wrote `new_tail=1`

## IO CQE Raw/Decode

Raw CQE marker:
- `[sexdrive.nvme.io.read.cqe] cid=69 phase=1 dw0=0x0 dw1=0x0 dw2=0x10001 dw3=0x10045`

Decode (corrected mapping):
- `CID = DW3[15:0] = 0x0045 = 69`
- `Phase = DW3[16] = 1`
- `StatusField = DW3[31:17] = 0`
- `SC = 0`, `SCT = 0` (success)
- `SQHD = DW2[15:0] = 1`
- `SQID = DW2[31:16] = 1`

CQ consume:
- `CQ1HDBL` advanced with `head+1` (`cqh=1`, `cqp=1`)

## Data Buffer First Bytes

From `[sexdrive.nvme.io.read.ok]`:
- `d0=0x0`
- `d1=0x0`
- `d2=0x0`
- `d3=0x0`

(Completion succeeded; first sampled bytes are zero in this lane.)

## Safety

- no `#PF`, `#GP`, `panic`
- typed `BLOCK_READ` path remains honest pre-API (`ERR_NO_DEVICE` behavior unchanged)

## Proof Markers

- `[sexdrive.nvme.io.read.begin]`
- `[sexdrive.nvme.io.read.submit]`
- `[sexdrive.nvme.io.read.doorbell]`
- `[sexdrive.nvme.io.read.cqe]`
- `[sexdrive.nvme.io.read.ok]`

## Files Changed

- `apps/sexdrive/src/main.rs`
- `docs/handoff/SEXDRIVE_NVME_IO_READ_ONE_BLOCK_PROOF_V1.md`

## Final Grep

```bash
SEXOS_GATE_NVME=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log \
| grep -E 'sexdrive\.nvme\.(admin\.identify\.v2|ioq|io\.read)|kernel\.pci\.nvme|kernel\.cap\.nvme_bar|#PF|#GP|panic'
```

## Next Prompt

`SEXDRIVE_BLOCK_READ_API_WIRE_V1`
