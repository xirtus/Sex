# SEXFILES_FAULT_INJECTION_GATE_V1

- date: 2026-05-06
- git commit: 0e3ff0e (dirty — pending commit)
- target: `SEXOS_SEXFILES_FAULT_INJECTION_PROOF=1`
- qemu: same as MASTER_RUNTIME_GATE_V1

## Purpose
Add a deterministic SexFiles fault-injection proof gate covering the full 12-point fault matrix:
corruption, bounds, replay, revocation, generation, and out-of-space behavior.
This is the "near-100% credibility" gate — it consolidates existing fault-path coverage
and adds the missing entry-level checksum mismatch check into a single pass.

## Gate Activation
```
SEXOS_SEXFILES_FAULT_INJECTION_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log
```

## Fault Matrix Results

| # | Test | Marker | Result |
|---|------|--------|--------|
| 1 | invalid object id rejected | `[sexfiles.fault.proof.invalid_object]` | PASS (ok=1) |
| 2 | table full rejected | `[sexfiles.fault.proof.table_full]` | PASS (ok=1) |
| 3 | journal full rejected | `[sexfiles.fault.proof.journal_full]` | PASS (ok=1) |
| 4 | oversized write rejected | `[sexfiles.fault.proof.oversized_write]` | PASS (ok=1) |
| 5 | corrupt journal record rejected | `[sexfiles.fault.proof.corrupt_reject]` | PASS (ok=1) |
| 6 | uncommitted transaction ignored | `[sexfiles.fault.proof.uncommitted_ignore]` | PASS (ok=1) |
| 7 | committed transaction replayed | `[sexfiles.fault.proof.committed_replay]` | PASS (ok=1) |
| 8 | revoked cap denied | `[sexfiles.fault.proof.revoked_deny]` | PASS (ok=1) |
| 9 | wrong owner/caller denied | `[sexfiles.fault.proof.owner_deny]` | PASS (ok=1) |
| 10 | generation rollback denied | `[sexfiles.fault.proof.generation_deny]` | PASS (ok=1) |
| 11 | checksum mismatch denied | `[sexfiles.fault.proof.checksum_mismatch]` | PASS (ok=1) |
| 12 | out-of-space deterministic error | `[sexfiles.fault.proof.out_of_space]` | PASS (ok=1) |
| – | pass summary | `[sexfiles.fault.proof.pass]` | ALL CHECKS PASSED |
| – | start marker | `[sexfiles.fault.proof.start]` | EMITTED |

## Implementation

### New DiskFs proof-injection methods (`diskfs.rs`)

1. **`proof_inject_bad_entry_checksum(object_id)`** — flips one checksum bit on a live
   object entry. Subsequent `stat_object_entry()` detects the mismatch and returns
   `ERR_OVERFLOW`. This is the only fault path in the matrix that was previously
   untested (journal-record checksum was tested; entry-level was not).

2. **`proof_fill_journal_and_test_full()`** — fills the journal to capacity with raw
   `TxBegin` records (bypassing table allocation, unlike `create_object_entry` which
   reaches table-full before journal-full). Then attempts one more normal
   `append_journal_record()`, which must return `ERR_FULL`. This isolates the
   journal-full path from the table-full path.

### Fault injection proof runner (`proof.rs`)

12 private `fault_*()` functions plus one public entry point:
- `run_sexfiles_fault_injection_proofs()` — orchestrates all 12 tests and emits the
  unified `[sexfiles.fault.proof.pass]` marker.

### Trampoline hook (`trampoline.rs`)

Standard `option_env!` gate matching existing proof hook patterns:
```rust
const SEXFILES_FAULT_INJECTION_PROOF_ENABLED: bool =
    option_env!("SEXOS_SEXFILES_FAULT_INJECTION_PROOF").is_some();
```

## Files Changed

| File | Change |
|------|--------|
| `servers/sexfiles/src/backends/diskfs.rs` | +new `proof_inject_bad_entry_checksum` + `proof_fill_journal_and_test_full` |
| `servers/sexfiles/src/proof.rs` | +`run_sexfiles_fault_injection_proofs` + 12 sub-functions (~308 lines) |
| `servers/sexfiles/src/trampoline.rs` | +gate hook for `SEXOS_SEXFILES_FAULT_INJECTION_PROOF` |

## Build/Runtime

- `cargo check -p sexfiles`: **PASS**
- `./scripts/entrypoint_build.sh`: **PASS**
- `SEXOS_SEXFILES_FAULT_INJECTION_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log`: **PASS (GREEN_MASTER)**

## Non-Goals Kept
- No kernel edits
- No `sex-pdx` ABI edits
- No app-visible debug/fault interface
- No nondeterministic timing required
- No real power loss required
- No shared-memory/backing-buffer redesign
- No broad refactor

## Contract Preserved
- Bounded synthetic fixtures only (DISKFS_MAX_OBJECTS=16, DISKFS_JOURNAL_CAPACITY=64)
- All fault injections are deterministic
- RamFS and DiskFS contract locks unchanged
- PDX message protocol unchanged
- Flat namespace, no POSIX semantics

## SexFiles Current Percentage Estimate

| Layer | Coverage | Notes |
|-------|----------|-------|
| RamFS contract (open/read/write/close/stat/list) | ~95% | All paths tested including bounds, OOB, owner, caps |
| DiskFS object table scaffold | ~85% | Format/mount/create/stat/table-full all tested |
| Append-only journal | ~80% | Begin/commit/checksum/full all tested |
| Replay/recovery | ~75% | Committed/uncommitted/corrupt/generation-order tested |
| Capability records (grant/revoke/generation) | ~90% | Grant/reject/revoke/stale-gen all tested |
| Fault injection (combined surface) | ~95% | All 12 fault paths deterministically verified |
| **SexFiles overall** | **~87%** | |

## Exact Blockers to 100%

1. **No persistent block-device I/O route** — DiskFs is a RAM scaffold.
   SexFiles↔SexDrive write/read of journal blocks and object table blocks
   is unimplemented. Without this, there is no real durability.

2. **No reboot-time replay integration** — `replay_journal_records()` works on
   synthetic in-memory slices. There is no path from persisted journal media
   into the replay engine at server startup.

3. **No checkpoint integration** — The superblock `fs_generation` increments
   per create but there is no checkpoint record selection, no periodic
   snapshot write, and no crash-consistent checkpoint restart.

4. **No capability/revocation persistence** — CapRecord grants and revocations
   live only in RamFS memory. They are not journaled/serialized to DiskFS.

## Next 3 Remaining Tasks

1. Wire SexFiles→SexDrive block write/read PDX channel for journal flush
   and object-table persistence (requires PDX block-IO protocol extension
   but NOT kernel/sex-pdx ABI edit — the server-level PDX path already exists).

2. Implement checkpoint record type (`TxCheckpoint`) in journal and
   integrate with superblock advancement, so replay can find the last
   good checkpoint and apply only post-checkpoint transactions.

3. Serialize CapRecord grants/revocations into journal metadata update
   payloads so capability state survives across reboots.
