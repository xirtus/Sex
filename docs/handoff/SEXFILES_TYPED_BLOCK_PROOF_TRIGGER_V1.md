# SEXFILES_TYPED_BLOCK_PROOF_TRIGGER_V1

- date: 2026-05-07
- proves: sexfiles → sexdrive SLOT_BLOCK typed block route end-to-end

## Summary

Activated the existing typed block proof in `run_sexfiles_real_block_proofs()` which was
already wired at the trampoline level but non-functional due to an IPC reply collection bug.

**Root cause**: `SLOT_BLOCK` is a `CapabilityData::Domain` capability, which uses
`GraphEdge::AsyncEnqueue`. `pdx_call(SLOT_BLOCK, ...)` enqueues to sexdrive's ring and
returns `(0, 0)` immediately — it does NOT block waiting for a reply. The original
`diskfs_block_call` used the `(status, value)` from `pdx_call` directly, which was always 0.

**Fix**: After `pdx_call(SLOT_BLOCK, ...)`, loop on `pdx_listen_raw(0)` until `type_id == 0x1`
(kernel IPC reply marker). sexdrive processes the command and calls `pdx_reply(caller_pd, status)`,
which routes through `send_reply` into `incoming_replies`. `pdx_listen_raw(0)` checks
`incoming_replies` with priority over `message_ring`, so the reply is returned as
`PdxMessage { type_id: 0x1, arg0: reply_status }`.

## Files Changed

### `servers/sexfiles/src/backends/diskfs.rs`

- Added `pdx_listen_raw, sys_yield` to sex_pdx import
- `diskfs_block_call`: changed from reading `pdx_call` return value to looping on
  `pdx_listen_raw(0)` until `type_id == 0x1`, returning `msg.arg0` as the block reply status

### `servers/sexfiles/build.rs` (new file)

Added `cargo:rerun-if-env-changed` declarations for all `option_env!` proof flags used in
`trampoline.rs`. Without this, cargo would use the cached sexfiles binary and not pick up
env var changes for incremental builds.

## IPC Path Diagram

```
sexfiles                    kernel              sexdrive
   |                           |                   |
   |-- pdx_call(SLOT_BLOCK) -->|                   |
   |                           |-- AsyncEnqueue --> |
   |<-- returns (0,0) ---------|                   |
   |                           |                   |
   |-- pdx_listen_raw(0) ----->|                   |
   |   [loop: type_id != 1]    |                   |
   |   sys_yield()             |                   |
   |                           |<-- pdx_try_listen_raw(0) -|
   |                           |    finds block msg        |
   |                           |<-- pdx_reply(pd11, 4) ----|
   |                           |-- send_reply(pd11, 4) -->  |
   |                           |   incoming_replies += 4    |
   |<-- type_id=0x1, arg0=4 ---|                   |
   |   BLOCK_ERR_NO_DEVICE                         |
```

## Observed Marker Chain (SEXOS_SEXFILES_REAL_BLOCK_PROOF=1)

```
[sexfiles.block.proof.start]
[sexfiles.block.proof.route] ok=1 block_size=4096 route=in_memory_scaffold
[sexfiles.block.proof.write] ok=1 offset=0 len=4096
[sexfiles.block.proof.read] ok=1 offset=4096 len=512
[sexfiles.block.proof.match] ok=1 magic=0x315653454c494653
[sexfiles.block.proof.bounds_deny] ok=1 max_block=4096
[sexfiles.block.proof.align_deny] ok=1 sector_size=512
[sexfiles.block.proof.route_demo] typed BLOCK_READ via SLOT_BLOCK=15
[sexfiles.diskfs.typed.call] cmd=BLOCK_READ offset=0x0 size=512 buf_cap=0x0
[sexdrive.block.typed.recv] cmd=1 offset=0x0 size=512 buf_cap=0x0 caller=11
[sexdrive.block.typed] cmd=1 ERR_NO_DEVICE honest=no_nvme_ahci_backend
[sexblock.abi.reply.encode] caller=11 status=4
[sexdrive.block.typed.reply] cmd=1 caller=11 status=4
[sexfiles.diskfs.typed.reply] cmd=BLOCK_READ status=4
[sexfiles.block.proof.typed_read] status=4 expected=ERR_NO_DEVICE(4)
[sexfiles.diskfs.typed.call] cmd=BLOCK_WRITE offset=0x0 size=512 buf_cap=0x0
[sexdrive.block.typed.recv] cmd=2 ... status=4
[sexfiles.diskfs.typed.reply] cmd=BLOCK_WRITE status=4
[sexfiles.block.proof.typed_write] status=4 expected=ERR_NO_DEVICE(4)
[sexfiles.diskfs.typed.call] cmd=BLOCK_SYNC
[sexdrive.block.typed.recv] cmd=3 ... status=4
[sexfiles.diskfs.typed.reply] cmd=BLOCK_SYNC status=4
[sexfiles.block.proof.typed_sync] status=4 expected=ERR_NO_DEVICE(4)
[sexdrive.block.typed.recv] cmd=255 ... ERR_BAD_CMD ... status=1
[sexfiles.block.proof.bad_cmd] reply=1 expected=ERR_BAD_CMD(1)
[sexfiles.diskfs.typed.call] cmd=BLOCK_READ offset=0x0 size=8192 buf_cap=0x0
[sexdrive.block.typed.recv] cmd=1 ERR_BAD_LEN size=8192 max=4096 ... status=2
[sexfiles.diskfs.typed.reply] cmd=BLOCK_READ status=2
[sexfiles.block.proof.bad_len] reply=2 expected=ERR_BAD_LEN(2)
[sexfiles.diskfs.typed.call] cmd=BLOCK_READ offset=0x1 size=512 buf_cap=0x0
[sexdrive.block.typed.recv] cmd=1 ERR_BAD_LEN offset=0x1 ... status=2
[sexfiles.diskfs.typed.reply] cmd=BLOCK_READ status=2
[sexfiles.block.proof.unaligned] reply=2 expected=ERR_BAD_LEN(2)
[sexfiles.block.proof.typed_summary] honest=1 read=1 write=1 sync=1 bad_cmd=1 bad_len=1 unaligned=1
[sexfiles.block.proof.blocker] status=TYPED_ABI_WIRED reason=no_real_nvme_ahci_backend_read_still_blocked
[sexfiles.block.proof.done] contract_validated=1 route=TYPED_ABI_SLOT_BLOCK blocker=REAL_DEVICE_BACKEND_MISSING
```

## Gate Results

Normal boot (no flag): unchanged — SEXFILES_GATE PASS, no block.proof markers.

Proof boot (`SEXOS_SEXFILES_REAL_BLOCK_PROOF=1`): all typed block markers present, `honest=1`.

Only remaining gate failure: CLOCK_GATE (pre-existing LAPIC, separate plan).

## Verify Command

```bash
# Build with proof enabled
SEXOS_SEXFILES_REAL_BLOCK_PROOF=1 ./scripts/entrypoint_build.sh

# Run gate and show typed block markers
SEXOS_SEXFILES_REAL_BLOCK_PROOF=1 ./scripts/master_runtime_gate.sh --skip-build --probe 25 --keep-log

# Verify full chain
grep -E 'block\.proof|diskfs\.typed|block\.typed|sexblock\.abi' .gate_master/serial.log
```

## Remaining Blockers

1. `[sexfiles.block.proof.blocker] status=TYPED_ABI_WIRED` — No real NVMe/AHCI backend. Block route proven honest; actual device I/O blocked until SEXDRIVE_BACKEND_REALITY_AUDIT_V1.
2. CLOCK_GATE — pre-existing LAPIC timer issue (LAPIC_TIMER_SFMASK_PREMORTEM_V1).

## Next: SEXDRIVE_BACKEND_REALITY_AUDIT_V1

Audit what hardware detection path sexdrive would need to serve real BLOCK_READ/WRITE from
an NVMe or AHCI device. Scope: sexdrive only. No ABI/kernel changes.
