# SEXBLOCK_BUFFER_LEND_CAP_IMPLEMENT_PHASE_A_V1

## Mission
Resume interrupted Phase A and prove MemLend buffer-cap handoff path builds and runs without Phase B NVMe fill.

## Result
PASS (Phase A)

- `build_payload.sh`: PASS
- `entrypoint_build.sh` inside runtime gate: PASS after ABI hash update
- Runtime proof markers for MemLend + Phase A copy: observed
- No `#PF/#GP/panic` in proof window
- Phase A only: `0xBB` pattern write via MemLend mapping, no NVMe fill for this path

## Files Changed
- `apps/sexdrive/src/main.rs`
- `crates/sex-pdx/src/lib.rs` (Claude partial carried forward)
- `kernel/src/syscalls/mod.rs` (Claude partial carried forward)
- `servers/sexfiles/src/proof.rs` (Claude partial carried forward)
- `sexos_build_spec.toml` (ABI hash sync)
- `docs/handoff/SEXBLOCK_BUFFER_LEND_CAP_IMPLEMENT_PHASE_A_V1.md`

## Compile Errors Fixed
- No Rust compile errors in this pickup slice.
- One gate blocker fixed: `abi_version_hash mismatch vs spec`.
- Updated once in `sexos_build_spec.toml` to computed value:
  - `f7d28624177e9f903515c633cc0266d39077afc16aab539f26a87c127ed477b4`

## Final Syscall ABI (Phase A)
From Claude partial + validation in runtime:

- `SYS_GRANT_MEM_LEND = 50`
  - args: `rdi=domain_slot`, `rsi=length`, `rdx=lend_slot`
  - enforced: `length == 4096`
  - allocates kernel page, maps producer VA with caller PD pkey
  - refuses occupied target slot
  - installs `MemLend` cap at target slot (`SLOT_BUF_LEND=17`)
  - returns producer VA (or `u64::MAX` on error)

- `SYS_MAP_MEM_LEND = 51`
  - args: `rdi=cap_slot`
  - resolves `MemLend` cap in caller PD
  - maps consumer VA with caller (consumer) PD pkey
  - returns consumer VA (or `u64::MAX` on error)

- `sex-pdx` additions used:
  - `SLOT_BUF_LEND = 17`
  - `sys_grant_mem_lend(...)`
  - `sys_map_mem_lend(...)`

## SexDrive Phase A Wiring Done
`apps/sexdrive/src/main.rs` `BLOCK_READ` path now:
- if `buf_cap == SLOT_BUF_LEND`:
  - require `size == 512`, else `ERR_BAD_LEN`
  - call `sys_map_mem_lend(SLOT_BUF_LEND)`
  - if map invalid (`0` or `u64::MAX`) -> `ERR_NO_DEVICE`
  - write `0xBB` into first 512 bytes using volatile writes
  - markers:
    - `[sexdrive.bufcap.map.ok] fill_va=...`
    - `[sexdrive.block.read.handoff.copy.ok] phase=A len=512`
  - reply `OK(0)` only after successful write
- no Phase B NVMe copy in this path

## Proof Markers Observed
From `.gate_master/serial.log`:

- `[kernel.memlend.grant.ok] va=0x400000356000 phys=0x102cb000 len=4096`
- `[kernel.memlend.map.ok] va=0x400000357000 len=4096`
- `[sexfiles.bufcap.alloc.ok] buf_va=0x400000356000`
- `[sexfiles.bufcap.grant.ok] slot=17`
- `[sexdrive.bufcap.map.ok] fill_va=0x400000357000`
- `[sexdrive.block.read.handoff.copy.ok] phase=A len=512`
- `[sexfiles.bufcap.verify.ok] phase=A overwritten=1 first_byte=0xbb reply=0`
- `[sexblock.bufcap.phase_a.ok]`

## Negative Typed Tests
Still passing in same run (`typed_summary honest=1`):
- bad cmd -> `ERR_BAD_CMD`
- bad len -> `ERR_BAD_LEN`
- unaligned -> `ERR_BAD_LEN`
- write/sync -> `ERR_NO_DEVICE`

## Notes
- `FINAL_SCORE` remains `RED_MASTER` due to unrelated `CLOCK_GATE` miss (`silkbar.clock.send`), not storage proof failure.

## Final Grep Command
```bash
grep -E "memlend|bufcap|phase_a|handoff|#PF|#GP|panic" .gate_master/serial.log
```

## Next Prompt
`SEXBLOCK_BUFFER_LEND_CAP_NVME_FILL_PHASE_B_V1`
