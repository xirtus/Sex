# SEXDRIVE_BLOCK_READ_DATA_HANDOFF_V1

## Mission Result
STOP FIRST

Safe payload handoff was **not** implemented because current `buffer_cap` in typed `BLOCK_READ` is not a real, verifiable cross-PD buffer capability in this path.

## 1) Existing buffer/handoff reality
- Typed call packing today:
  - `opcode` = `BLOCK_READ`/`BLOCK_WRITE`/`BLOCK_SYNC`
  - `arg0` = `offset`
  - `arg1` = `size`
  - `arg2` = `buffer_cap`
- `servers/sexfiles/src/backends/diskfs.rs` explicitly documents `buffer_cap` as placeholder (`0 = no DMA buffer yet`).
- Current proof run still uses `buffer_cap=0` for read path.
- SexDrive currently performs real NVMe read into a local bounce buffer and returns status.

## 2) Why safe copy is blocked
- No existing primitive in this flow lets SexDrive:
  - validate `buffer_cap` ownership/lifetime/capacity for caller PD,
  - safely map/resolve `buffer_cap` via capability authority for block transfer,
  - enforce bounded copy target semantics by capability metadata.
- Using raw cross-PD pointer semantics from `arg2` would violate this mission constraints.

Classification:
- `BUFFER_CAP_NOT_REAL` (current route is status-only for payload semantics).

## 3) Runtime proof markers
Observed in `.gate_master/serial.log`:
- `[sexfiles.realread.payload.begin] mode=status_only`
- `[sexdrive.block.read.handoff.begin] offset=0x0 size=512 buf_cap=0x0`
- `[sexdrive.block.read.handoff.err] reason=buffer_cap_not_real buf_cap=0x0`
- `[sexfiles.realread.payload.err] reason=buffer_cap_not_real status=0`
- Existing status chain still passes:
  - `sexdrive.block.read.api.nvme.submit`
  - `sexdrive.block.read.api.cqe`
  - `sexdrive.block.read.api.ok`
  - `sexfiles.diskfs.typed.reply status=0`

## 4) Negative typed cases
Still passing:
- `BLOCK_WRITE -> ERR_NO_DEVICE`
- `BLOCK_SYNC -> ERR_NO_DEVICE`
- `bad cmd -> ERR_BAD_CMD`
- `bad len -> ERR_BAD_LEN`
- `unaligned offset -> ERR_BAD_LEN`

## 5) Files changed
- `apps/sexdrive/src/main.rs`
- `servers/sexfiles/src/proof.rs`
- `docs/handoff/SEXDRIVE_BLOCK_READ_DATA_HANDOFF_V1.md`

## 6) Build/gate result
- Build: PASS
- Storage proof chain: PASS (status-only)
- No `#PF/#GP/panic` in this mission window
- `FINAL_SCORE=RED_MASTER` remains due to unrelated `CLOCK_GATE` not storage regression.

## 7) Final grep command
```bash
SEXOS_GATE_NVME=1 SEXOS_SEXFILES_REAL_BLOCK_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log \
| grep -E 'sexdrive\\.block\\.read\\.(api|handoff)|sexfiles\\.realread\\.(status_ok|payload)|sexfiles\\.diskfs\\.typed\\.reply|#PF|#GP|panic'
```

## 8) Next prompt
`SEXBLOCK_BUFFER_CAP_HANDOFF_ABI_V1`

Needs:
- minimal, explicit capability-backed payload handoff contract
- producer/consumer buffer ownership + size metadata validation
- bounded copy semantics without shared-memory redesign
