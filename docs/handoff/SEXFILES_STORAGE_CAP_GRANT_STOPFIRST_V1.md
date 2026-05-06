# SEXFILES_STORAGE_CAP_GRANT_STOPFIRST_V1

## Root Cause
Storage denial was caused by missing capability grants, not ABI mismatch:

1. `sexfiles` server was live (PD 11) after boot deploy.
2. `SLOT_STORAGE` (`sex_pdx::SLOT_STORAGE = 1`) already existed.
3. Quil called correct storage route (`SLOT_STORAGE` + `OP_RAMFS_*`) but lacked slot capability.
4. Kernel grant phase had no `SLOT_STORAGE -> sexfiles` capability for Quil/Linen.

## STOP FIRST Resolution Applied (Approved)
Kernel edit was required and approved.

### Minimal kernel patch
File: `kernel/src/init.rs`

- Added `SLOT_STORAGE` grants to SexFiles domain for:
  - Linen PD (`[kernel.cap.storage.linen]`)
  - Quil PD (`[kernel.cap.storage.quil]`)
- Kept silk-shell without storage authority.
- No `sex-pdx` ABI edits.
- No broad grant redesign.

## Userland Proof Gate
Added proof gate:
- `SEXOS_STORAGE_CAP_PROOF=1`

### Markers implemented
- `[sexfiles.cap.proof.grant]`
- `[sexfiles.cap.proof.deny]`
- `[quil.storage.cap.ok]` (or blocker)
- `[linen.storage.cap.ok]` (or blocker)

### Where
- `servers/silk-shell/src/main.rs`
  - synthetic proof stage emits grant/deny markers
  - emits Linen exact blocker marker because Linen has no direct storage route implementation in this phase
- `servers/quil/src/main.rs`
  - storage cap probe on boot under proof gate
  - emits `[quil.storage.cap.ok]` on success

## Files Changed
- `kernel/src/init.rs`
- `servers/quil/src/main.rs`
- `servers/silk-shell/src/main.rs`
- `docs/handoff/SEXFILES_STORAGE_CAP_GRANT_STOPFIRST_V1.md`
- `docs/handoff/MASTER_RUNTIME_GATE_V1.md` (auto-updated by gate)

## Proof Markers Observed (Runtime)
From `.gate_master/serial.log`:
- `[kernel.cap.storage.linen] linen->sexfiles slot=1`
- `[kernel.cap.storage.quil] quil->sexfiles slot=1`
- `[quil.storage.cap.ok] status=0 handle=0`
- `[sexfiles.cap.proof.grant] sid=420 ok=1`
- `[sexfiles.cap.proof.deny] sid=421 ok=1`
- `[linen.storage.cap.blocker] reason=no_linen_storage_route shell_status=0xfffffffffffffffc`

## Build / Runtime Result
- `./scripts/entrypoint_build.sh`: PASS
- `SEXOS_STORAGE_CAP_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log`: PASS (`GREEN_MASTER`)
- `SEXFILES_GATE`: PASS (`sexfiles.ready`, `kernel.spawn.sexfiles`, `task.running pd_id=11`)

## Remaining Storage Authority Risks
1. Linen now has storage slot capability but no direct storage-call route yet; marker currently reports explicit route blocker.
2. Quil proof currently validates open/close reachability only; deeper read/write policy proof stays in SexFiles/Quil contract prompts.
3. Capability model remains static boot grants; no dynamic delegation/revocation in this patch.
