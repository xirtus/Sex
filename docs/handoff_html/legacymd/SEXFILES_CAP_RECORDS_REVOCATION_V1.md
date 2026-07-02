# SEXFILES_CAP_RECORDS_REVOCATION_V1

## Purpose
Add and enforce a bounded SexFiles capability-record model (PD subject + rights bits + generation) in the active `RamFs` path, with revocation via generation bump and stale-cap denial.

## Cap Record Layout
Implemented in `servers/sexfiles/src/backends/ramfs.rs`:
- `object_id: u64`
- `subject_pd: u32`
- `rights: u8` bitmask
- `generation: u64`
- `valid: bool`

Rights bits:
- `READ`
- `WRITE`
- `APPEND`
- `LIST`
- `DELETE`
- `GRANT`

Bounded table:
- `CAP_MAX_RECORDS = 256`
- deterministic `ERR_FULL` on overflow

## Rights Enforced
Owner fast-path kept (explicit):
- `caller_pd == 0` (server internal) OR `caller_pd == owner_pd` => allow

Capability checks for non-owner callers:
- `open(existing)` => any of `{READ, WRITE, APPEND, DELETE, GRANT}`
- `read` => `READ`
- `write` => `WRITE` (or `APPEND` when offset is at/after current end)
- `close` => `READ` (minimum capability to close active handle)
- `stat` => `READ`
- `list`/`len` visibility => `LIST`

## Revocation Behavior
- Each object has `cap_generation` in file entry.
- Revocation path bumps object generation and invalidates related cap records.
- Access checks require cap generation to match object generation.
- Stale generation cap records are denied (`ERR_PERM_DENIED`).

## Journal Interaction
- Existing append-only journal/replay is currently in `DiskFs` scaffold.
- Active capability enforcement path is `RamFs` (live VFS route), so cap updates are not persisted to disk yet.
- This is an explicit persistence gap, not claimed as durable.

## Files Changed
- `servers/sexfiles/src/backends/ramfs.rs`
- `servers/sexfiles/src/proof.rs`
- `servers/sexfiles/src/trampoline.rs`
- `docs/handoff/SEXFILES_CAP_RECORDS_REVOCATION_V1.md`

## Proof Gate / Markers
Gate:
- `SEXOS_SEXFILES_CAP_RECORD_PROOF=1`

Markers:
- `[sexfiles.caprec.proof.read_allow]`
- `[sexfiles.caprec.proof.write_allow]`
- `[sexfiles.caprec.proof.missing_deny]`
- `[sexfiles.caprec.proof.revoked_deny]`
- `[sexfiles.caprec.proof.generation_deny]`
- `[sexfiles.caprec.proof.grant_allow]`

Runtime evidence:
- All markers emitted with `ok=1`.

## Build / Runtime
- `cargo check --target sex-src/targets/x86_64-unknown-sexos.json -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem -p sexfiles`: PASS
- `./scripts/entrypoint_build.sh`: PASS
- `SEXOS_SEXFILES_CAP_RECORD_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log`: PASS (`GREEN_MASTER`)

## Remaining Collar Integration Risks
1. Collar remains shell-side policy; SexFiles cap records are server-local enforcement only.
2. No shared signed capability token format across servers yet.
3. No durable cap record persistence/replay in live RamFs path.
4. GRANT operation is proof-helper/internal path here; no new public ABI for external grant delegation yet.
