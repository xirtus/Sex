# SEXFILES_SEXDRIVE_REAL_READ_PROOF_V1

## Mission
Prove SexFiles sends typed `BLOCK_READ` via `SLOT_BLOCK` and receives `OK` only after real NVMe IO completion in SexDrive.

## Date
2026-05-07

## Files Changed
- `servers/sexfiles/src/proof.rs`
- `scripts/master_runtime_gate.sh`

## Proof Trigger Path
- Build/run with:
  - `SEXOS_GATE_NVME=1`
  - `SEXOS_SEXFILES_REAL_BLOCK_PROOF=1`
- `sexfiles` proof trampoline gate:
  - `option_env!("SEXOS_SEXFILES_REAL_BLOCK_PROOF").is_some()`
  - invokes `run_sexfiles_real_block_proofs()`

## Runtime Evidence (Serial)
Observed marker chain:
- `[sexfiles.realread.begin]`
- `[sexfiles.diskfs.typed.call] cmd=BLOCK_READ offset=0x0 size=512 buf_cap=0x0`
- `[sexdrive.block.read.api.recv] offset=0x0 size=512 buf_cap=0x0`
- `[sexdrive.block.read.api.nvme.submit] cid=1280 nsid=1 slba=0 nlb=0 prp1=0x102c7000 sq_tail=1`
- `[sexdrive.block.read.api.cqe] cid=1280 phase=1 dw2=0x10002 dw3=0x10500`
- `[sexdrive.block.read.api.ok] cid=1280 slba=0 nlb=0 d0=0x0 d1=0x0`
- `[sexfiles.diskfs.typed.reply] cmd=BLOCK_READ status=0`
- `[sexfiles.realread.status_ok] ok=1`
- `[sexfiles.realread.payload_not_wired] ok=1`

This proves `BLOCK_READ` returns `OK` only after a real NVMe CQE success path.

## Status Chain Result
- SexFiles typed call -> SexDrive typed recv -> NVMe submit -> NVMe CQE success -> typed reply status `0` -> SexFiles status OK marker.

## Payload Handoff Status
- Still missing by design in this mission.
- Current behavior is bounce-buffer-only in SexDrive.
- Explicitly logged:
  - `[sexfiles.realread.payload_not_wired] ok=1`
  - `[sexfiles.block.proof.blocker] status=TYPED_READ_OK_WITHOUT_PAYLOAD_HANDOFF reason=sexdrive_bounce_buffer_only`

## Negative Typed Cases
All preserved and passing in proof run:
- Bad command -> `ERR_BAD_CMD(1)`
- Bad length (`size > 4096`) -> `ERR_BAD_LEN(2)`
- Unaligned offset (`offset % 512 != 0`) -> `ERR_BAD_LEN(2)`
- BLOCK_WRITE -> `ERR_NO_DEVICE(4)`
- BLOCK_SYNC -> `ERR_NO_DEVICE(4)`
- Summary marker: `[sexfiles.block.proof.typed_summary] honest=1 ...`

## Gate Result Context
- `SEXFILES_GATE=PASS`
- `FINAL_SCORE=RED_MASTER` only because `CLOCK_GATE` failed (`silkbar.clock.send` not observed in this window).
- No NVMe/block-route regression in this mission.

## Final Grep Command
```bash
SEXOS_GATE_NVME=1 SEXOS_SEXFILES_REAL_BLOCK_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log \
| grep -E 'sexfiles\.(realread|diskfs\.typed|block\.proof)|sexdrive\.block\.read\.api|sexdrive\.block\.typed|sexblock\.abi|kernel\.pci\.nvme|kernel\.cap\.nvme_bar|#PF|#GP|panic'
```

## Next Prompt
- `SEXDRIVE_BLOCK_READ_DATA_HANDOFF_V1`
- Optional guard follow-up: `SEXDRIVE_NVME_WRITE_GUARD_V1`
