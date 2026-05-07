# SEXDRIVE_NVME_ADMIN_QUEUE_REPROVISION_V1

- date: 2026-05-07
- scope: disable/reprovision/enable admin queue only
- files touched: `apps/sexdrive/src/main.rs`, this handoff
- no Identify submit in this mission
- no kernel/syscall/ABI/storage changes

## Summary

Reprovision succeeded.

Sequence completed safely:
1. Cleared `CC.EN`.
2. Observed `CSTS.RDY -> 0`.
3. Allocated/mapped SexDrive-owned ASQ/ACQ pages.
4. Programmed `AQA/ASQ/ACQ` to new values.
5. Set `CC.EN`.
6. Observed `CSTS.RDY -> 1`.
7. Verified readback matches expected values.

## Old State

- `CC = 0x460001`
- `CSTS = 0x1`
- `AQA = 0x00ff003f`
- `ASQ = 0x1ffdd000`
- `ACQ = 0x1ffde000`
- `CAP.TO = 15`
- `CAP.DSTRD = 0`

## Disable Result

- Wrote `CC = 0x460000` (`EN=0`, other fields preserved).
- `RDY=0` reached.
- Poll count: `disable_polls=0`.

## New Queue Allocation

- `ASQ phys=0x1f91c000 va=0x400000005000`
- `ACQ phys=0x102ab000 va=0x400000006000`
- Both pages are 4096-byte aligned and zeroed before programming.

## Programmed New Queue Registers

- `AQA = 0x000f000f` (ASQS=15, ACQS=15 => 16 entries each)
- `ASQ = 0x1f91c000`
- `ACQ = 0x102ab000`

## Enable Result

- Wrote `CC = 0x460001` (`EN=1` restored).
- `RDY=1` reached.
- Poll count: `enable_polls=0`.

## Readback Verification

- `CC = 0x460001`
- `CSTS = 0x1`
- `AQA = 0x000f000f`
- `ASQ = 0x1f91c000`
- `ACQ = 0x102ab000`

All expected values matched.

## Proof Markers Observed

- `[sexdrive.nvme.reprovision.begin]`
- `[sexdrive.nvme.reprovision.disable.begin]`
- `[sexdrive.nvme.reprovision.rdy0]`
- `[sexdrive.nvme.reprovision.alloc.ok]`
- `[sexdrive.nvme.reprovision.program.aqa]`
- `[sexdrive.nvme.reprovision.program.asq]`
- `[sexdrive.nvme.reprovision.program.acq]`
- `[sexdrive.nvme.reprovision.enable.begin]`
- `[sexdrive.nvme.reprovision.rdy1]`
- `[sexdrive.nvme.reprovision.ok]`

No `#PF`, `#GP`, or `panic` found in this run.

## Final Proof Grep

```bash
SEXOS_GATE_NVME=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log \
| grep -E 'sexdrive\.nvme\.(bar\.resolve|identity|queue|adminq|reprovision)|kernel\.pci\.nvme|kernel\.cap\.nvme_bar|#PF|#GP|panic'
```

## Next Prompt

`SEXDRIVE_NVME_ADMIN_IDENTIFY_RETRY_V1`
