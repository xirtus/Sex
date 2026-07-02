# SEXFILES_SNAPSHOT_CHECKPOINT_V1

## Purpose
Implement minimal SexFiles checkpoint/snapshot records for object-table generations.
Captures the full object table + superblock generation at a point in time into a
bounded checkpoint slot. Supports create, find-latest-valid, restore, and
corrupted-checkpoint-skip. NOT a Btrfs snapshot — no recursive subvolumes, no
POSIX paths, no clone semantics.

## Checkpoint Structure (Implemented)

### Constants
- `DISKFS_MAX_CHECKPOINTS = 4` — fixed bound on checkpoint slots
- `DISKFS_CHECKPOINT_MAGIC = 0x4348_4B50_4E54_5631` — "CHKPNTV1"

### `SexfilesCheckpoint` struct
| Field                    | Type                                            | Purpose                                |
|--------------------------|-------------------------------------------------|----------------------------------------|
| `magic`                  | `u64`                                           | Magic constant for validation          |
| `checkpoint_generation`  | `u64`                                           | Monotonic generation ID                |
| `fs_generation`          | `u64`                                           | Superblock generation at snapshot time |
| `table`                  | `[SexfilesObjectEntry; DISKFS_MAX_OBJECTS]`     | Full object table snapshot             |
| `checksum`               | `u32`                                           | Integrity checksum over all fields     |
| `valid`                  | `bool`                                          | Whether this slot is occupied          |

### Checksum Algorithm
XOR-based hash over: magic halves, both generation halves, plus per-entry
checksums for all in-use entries with position-dependent mixing (index << 16).
Same deterministic XOR family as superblock/entry/journal checksums.

## Operations

### `create_checkpoint() -> Result<u64, i64>`
- Reads current object table + fs_generation under shared lock
- Finds next free slot or overwrites oldest (lowest checkpoint_generation)
- Computes checksum, stores, bumps `next_checkpoint_generation`
- Returns the new checkpoint's generation number

### `find_latest_valid_checkpoint() -> Option<(usize, SexfilesCheckpoint)>`
- Scans all slots, validates magic + checksum
- Returns the slot index + checkpoint with highest `checkpoint_generation`
- Skips corrupted checkpoints (bad magic or mismatched checksum)
- Returns `None` if no valid checkpoint exists

### `restore_checkpoint(cp: &SexfilesCheckpoint) -> Result<u64, i64>`
- Validates magic + checksum before restoration
- Overwrites active object table with checkpoint's table
- Advances superblock `fs_generation` to `cp.fs_generation + 1`
- Returns the restored checkpoint's generation

### Proof Scenarios
- `proof_corrupt_skip_scenario()` — creates 2 checkpoints, corrupts higher-gen one,
  verifies `find_latest_valid` returns lower-gen valid checkpoint
- `proof_generation_monotonic_scenario()` — verifies gen₁ < gen₂ monotonic increase
  and correct fs_generation advancement on restore
- `proof_checkpoint_roundtrip()` — full create→checkpoint→mutate→restore→verify cycle

## Files Changed
- `servers/sexfiles/src/backends/diskfs.rs` — checkpoint struct, constants, checksum,
  create/find/restore/proof methods, state fields, format reset
- `servers/sexfiles/src/proof.rs` — 6 proof functions under checkpoint proof gate
- `servers/sexfiles/src/trampoline.rs` — `SEXOS_SEXFILES_CHECKPOINT_PROOF` gate wiring
- `docs/handoff/SEXFILES_SNAPSHOT_CHECKPOINT_V1.md` — this handoff

## Proof Gate / Markers
Gate:
- `SEXOS_SEXFILES_CHECKPOINT_PROOF=1`

Markers:
- `[sexfiles.checkpoint.proof.create]` — checkpoint creation succeeds, gen ≥ 1
- `[sexfiles.checkpoint.proof.latest_valid]` — latest valid found with correct gen
- `[sexfiles.checkpoint.proof.restore]` — restore removes post-checkpoint objects
- `[sexfiles.checkpoint.proof.corrupt_skip]` — corrupt high-gen skipped, lower gen used
- `[sexfiles.checkpoint.proof.generation]` — monotonic generation, fs_gen advanced
- `[sexfiles.checkpoint.proof.roundtrip]` — full e2e: create→checkpoint→mutate→restore→verify
- `[sexfiles.checkpoint.proof.done]` — all checks passed

## Build / Runtime
- `cargo check --target x86_64-sex.json -Z build-std=core,alloc -Z build-std-features=compiler-builtins-mem -p sexfiles` : pending target installation
- `SEXOS_SEXFILES_CHECKPOINT_PROOF=1` activates the proof gate at compile time

## Snapshot / Checkpoint Limits (Explicit)
| Limit                    | Value | Reason                                       |
|--------------------------|-------|----------------------------------------------|
| Max checkpoint slots     | 4     | Bounded static memory; oldest overwritten    |
| Objects captured         | ≤16   | DISKFS_MAX_OBJECTS table bound               |
| Checkpoint depth         | flat  | No recursive subvolumes                      |
| Block device persistence | NONE  | In-memory scaffold only; no real block route |
| POSIX paths              | NONE  | Strictly object-table metadata records       |
| Btrfs clone              | NONE  | Not in scope                                 |
| Kernel edits             | 0     | No kernel changes required                   |
| sex-pdx ABI edits        | 0     | No ABI changes required                      |

## Non-Goals (Kept)
- No recursive subvolumes
- No Btrfs clone or reflink
- No POSIX path semantics
- No kernel edits
- No `sex-pdx` ABI edits
- No broad metadata tree rewrite
- No shared-memory/backing-buffer redesign
- No real block device persistence (still in-memory scaffold only)
- No named read-only snapshot view (optional future task)

## STOPS Enforced
- [x] No broad metadata tree rewrite
- [x] No kernel edit
- [x] No sex-pdx ABI edit
- [x] No Btrfs-scale snapshot scope
- [x] No persistence claim exceeding actual in-memory backend
