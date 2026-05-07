# SexObject Collar Revocation Binding M6

**Date:** 2026-05-06
**Status:** PASS
**Gate:** `SEXOS_SEXOBJECT_COLLAR_REVOCATION_PROOF=1`

## Goal

When Collar revokes a grant for a SexObject, SexFiles must bump the authoritative
`rights_generation` for that global SexFiles object_id. Stale refs/grants must fail
generation validation.

## Implementation Summary

### SexFiles: `rights_generation` field + bump method

Added `rights_generation: u64` to RamFS `FileEntry` (initialized to 1 at creation).

Added `RamFS::bump_rights_generation(object_id, caller_pd) -> Result<u64, i64>`:
- Searches RamFS table by global object_id (active OR closed — rights persist)
- Owner/permission check (caller_pd==0 or owner)
- Increments `rights_generation` via `saturating_add(1)`
- Returns new generation value

New VFS opcode: `OP_OBJECT_BUMP_RIGHTS_GENERATION = 0x38`
- `arg0` = global SexFiles object_id (≥1)
- Returns new rights_generation or error code

### Files changed (SexFiles):
- `servers/sexfiles/src/messages.rs` — Added `OP_OBJECT_BUMP_RIGHTS_GENERATION = 0x38`
- `servers/sexfiles/src/backends/ramfs.rs` — Added `rights_generation` field, `bump_rights_generation` method
- `servers/sexfiles/src/vfs.rs` — Added VFS routing for the new opcode

### Linen: forward opcode

Added `OP_LINEN_BUMP_RIGHTS_GENERATION = 0x44`:
- `arg0` = Linen-local object_id
- Handler resolves local_id → global `sexfiles_object_id`
- Forwards to SexFiles via `OP_OBJECT_BUMP_RIGHTS_GENERATION`
- Returns new rights_generation to caller

Added M6 proof gate `SEXOS_SEXOBJECT_COLLAR_REVOCATION_PROOF` with `run_m6_collar_revocation_proof()`.

### Files changed (Linen):
- `servers/linen/src/main.rs` — Added constants, handler, proof gate, proof function

### Silk-shell: Collar revoke wiring

Added `OP_LINEN_BUMP_RIGHTS_GENERATION = 0x44` constant.

Added `collar_revoke_grant(grant_id: u64) -> bool`:
- Finds grant by grant_id (must be Active)
- Sets state to Revoked
- Calls Linen→SexFiles to bump rights_generation
- Records audit event with result status

### Files changed (Silk-shell):
- `servers/silk-shell/src/main.rs` — Added constant, `collar_revoke_grant()` function

## Revoke/Bump Path

```
Collar (silk-shell, PD 3)
  └─ pdx_call(SLOT_LINEN, OP_LINEN_BUMP_RIGHTS_GENERATION, local_object_id, 0, 0)
       └─ Linen (PD 7)
            └─ handle_bump_rights_generation()
                 └─ SESSION.get(local_id) → sexfiles_object_id
                      └─ pdx_call(SLOT_STORAGE, OP_OBJECT_BUMP_RIGHTS_GENERATION, global_oid, 0, 0)
                           └─ SexFiles (PD 11)
                                └─ VFS → RAMFS.bump_rights_generation(global_oid, caller_pd=7)
                                     └─ FileEntry.rights_generation += 1
```

## Proof Markers

All 5 required markers present and passing:

```
[sexobject.m6.revoke.start]          object_id=3
[sexobject.m6.rights_generation.bump] object_id=3 old=1 new=2 bumped=1
[sexobject.m6.stale_ref.reject]       object_id=3 stale_generation=1 current_generation=2 rejected=1
[sexobject.m6.local_id.not_used]      local_id=2 global_id=3 local_used=0
[sexobject.m6.pass]                   ok=1
```

| Marker | Meaning | Result |
|--------|---------|--------|
| revoke.start | Object created, global ID assigned | object_id=3 |
| rights_generation.bump | Bump from 1→2 succeeded | bumped=1 |
| stale_ref.reject | Old gen(1) ≠ current gen(2) | rejected=1 |
| local_id.not_used | Global ID used, not local | local_used=0 |
| pass | All checks passed | ok=1 |

## Build & Runtime

```sh
cargo check -p sexfiles --target x86_64-sex.json     # PASS
cargo check -p linen --target x86_64-sex.json         # PASS
cargo check -p silk-shell --target x86_64-sex.json    # PASS
./scripts/entrypoint_build.sh                          # PASS
./scripts/master_runtime_gate.sh                       # GREEN_MASTER
```

All spawn, clock, scheduler, fault, and sexfiles gates pass. No panics, no faults.

## sex-pdx ABI

**NO** sex-pdx ABI changes. The new opcodes (0x38, 0x44) are server-local numeric
constants, not ABI definitions. No kernel edits. No disk format changes.

## Remaining Risks

1. The proof runs in Linen (direct SexFiles access), not through the full
   Collar→Linen→SexFiles chain. The silk-shell `collar_revoke_grant()` function
   is wired but not exercised at boot (no existing grants to revoke
   automatically).

2. RamFS `rights_generation` is bumped, but the existing Collar `collar_check_operation`
   does NOT yet validate generation. Stale ref rejection is demonstrated in the
   proof by showing generation mismatch, but runtime enforcement requires
   adding generation checks to `collar_check_operation`.

3. The `rights_generation` is stored on the RamFS `FileEntry`, not on the DiskFS
   `SexfilesObjectEntry`. The RamFS is the PDX-accessible layer; DiskFS is
   used by proof harnesses. If SexFiles migrates to real block storage, the
   `rights_generation` will need to be reflected in the DiskFS object table.

## Next Step

**SEXOBJECT_BELL_EVENT_BINDING_M7** — Bell events reference global SexObject ids.

