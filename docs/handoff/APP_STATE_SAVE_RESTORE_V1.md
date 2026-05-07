# APP_STATE_SAVE_RESTORE_V1

## Status: PASS — All 5 proof markers proven

- date: 2026-05-06
- git commit: (pending)
- gate: SEXOS_APP_STATE_SAVE_RESTORE_PROOF=1
- result: ALL CHECKS PASSED (GREEN_MASTER)

## Summary

Implemented a bounded, deterministic app runtime state save/restore cycle
using SexFiles RamFS as the backing store. The proof demonstrates that an
`AppStateRecord` (packed, versioned, checksummed) can be saved to a named
RamFS file, loaded back, validated, and verified for roundtrip integrity.
Stale generation detection and bounds enforcement are also proven.

### What Is Proven (Single-Boot)

- **Save**: Create AppStateRecord with app_id=42, gen=1, 26 bytes of state data → write all 53 serialized bytes to RamFS file
- **Load**: Open file, read 53 bytes back
- **Restore**: Deserialize, validate magic+version+checksum, verify all 6 fields match original
- **Stale reject**: Record with gen=2 rejected when gen >= 3 is required (ERR_PERM_DENIED)
- **Bounds**: Record creation with 33 bytes of data rejected when max is 32 (ERR_OVERFLOW); max-size (32) creation succeeds

### Honest Limitation

Save/load operates through the RamFS in-memory backend. State does NOT survive
a QEMU process restart (no real block device route exists). This is the same
persistence boundary documented in `SEXFILES_REAL_BLOCK_BACKEND_V1.md` and
`SEXFILES_REBOOT_PERSISTENCE_HARNESS_V1.md`. The save/restore contract is
correct; true disk persistence awaits the block device route.

## State Object Shape

### `AppStateRecord` (53 bytes, repr(C, packed))

| Offset | Field | Type | Size | Description |
|--------|-------|------|------|-------------|
| 0 | magic | u32 | 4 | 0x41535441 ("ASTA") |
| 4 | version | u8 | 1 | Schema version (current=1) |
| 5 | app_id | u16 | 2 | Owning app identifier |
| 7 | generation | u64 | 8 | Monotonic counter |
| 15 | flags | u8 | 1 | FLAG_ACTIVE (0x01) etc. |
| 16 | data_len | u8 | 1 | Actual data bytes (0..32) |
| 17 | data | [u8;32] | 32 | Bounded state payload |
| 49 | checksum | u32 | 4 | XOR over magic..data_len |

Constants:
- `APPSTATE_MAGIC = 0x4153_5441`
- `APPSTATE_VERSION = 1`
- `APPSTATE_MAX_DATA = 32`
- `APPSTATE_RECORD_SIZE = 53`

### Validation Rules

1. `magic != APPSTATE_MAGIC` → `ERR_INVALID_HANDLE`
2. `version > APPSTATE_VERSION` → `ERR_OVERFLOW` (future version rejected)
3. `data_len > APPSTATE_MAX_DATA` → `ERR_OVERFLOW`
4. `checksum != recomputed` → `ERR_OVERFLOW` (tamper/corrupt detected)
5. `generation < expected_gen` → `ERR_PERM_DENIED` (stale generation)

### Storage Medium

AppStateRecord is stored as raw bytes in a RamFS file (≤ 4096 bytes).
File name fits the RamFS 24-byte name bound. Write/read uses the existing
`OP_RAMFS_OPEN`/`OP_RAMFS_WRITE`/`OP_RAMFS_READ`/`OP_RAMFS_CLOSE` PDX ops
via `SLOT_STORAGE` → sexfiles (PD 11).

## Proof Markers

All 5 required markers pass:

| Marker | Status | Sample Output |
|--------|--------|---------------|
| `[app.state.proof.save]` | PASS | ok=1 app_id=42 generation=1 data_len=26 record_size=53 |
| `[app.state.proof.load]` | PASS | ok=1 bytes_read=53 expected=53 |
| `[app.state.proof.restore]` | PASS | ok=1 magic=0x41535441 version=1 app_id=42 generation=1 data_len=26 checksum_ok=1 |
| `[app.state.proof.stale_reject]` | PASS | ok=1 record_gen=2 expected_gen=3 |
| `[app.state.proof.bounds]` | PASS | ok=1 max_data=32 attempted=33 |

Additional diagnostic:

| Marker | Purpose |
|--------|---------|
| `[app.state.proof.start]` | Proof begin |
| `[app.state.proof.bounds.max]` | Max-size record creation success |
| `[app.state.proof.done]` | Proof complete |

## Files Changed

| File | Change |
|------|--------|
| `servers/sexfiles/src/appstate.rs` | **New** — AppStateRecord struct, pack/validate/checksum/generation check |
| `servers/sexfiles/src/proof.rs` | Added `run_app_state_save_restore_proofs()` + imports |
| `servers/sexfiles/src/trampoline.rs` | Added `SEXOS_APP_STATE_SAVE_RESTORE_PROOF` gate hook |
| `servers/sexfiles/src/lib.rs` | Added `pub mod appstate;` |
| `servers/sexfiles/src/main.rs` | Added `mod appstate;` (binary target) |
| `docs/handoff/APP_STATE_SAVE_RESTORE_V1.md` | This handoff document |

## Files NOT Changed (Per Mission Rules)

| File | Reason |
|------|--------|
| `crates/sex-pdx/src/lib.rs` | STOP FIRST — no new PDX opcodes needed |
| `kernel/src/` | STOP FIRST — no kernel changes |
| `crates/sex-object-model/src/lib.rs` | NOT CHANGED — `SexObjectKind::AppState = 2` already exists; AppStateRecord is a sexfiles-internal serialization type, not a model type |
| `servers/silk-shell/` | No shell lifecycle changes |

## Build/Runtime Result

### Compilation
```
cargo check -p sexfiles --target x86_64-sex.json
```
Result: PASS (no warnings, no errors)

### Full Build
```
SEXOS_APP_STATE_SAVE_RESTORE_PROOF=1 cargo build -p sexfiles --target x86_64-sex.json --release
```
Result: PASS

### Gate Run
```
SEXOS_APP_STATE_SAVE_RESTORE_PROOF=1 ./scripts/master_runtime_gate.sh --probe 15 --keep-log
```
Result: GREEN_MASTER — all 5 proof markers present and passing

## Usage Pattern For Apps

An app that wants save/restore would:

```rust
// SAVE (app-side, via PDX to sexfiles/SLOT_STORAGE)
let rec = AppStateRecord::create(app_id, generation, flags, &state_bytes)?;
let bytes = rec.as_bytes();
// Write bytes to RamFS file via OP_RAMFS_OPEN+OP_RAMFS_WRITE+OP_RAMFS_CLOSE

// RESTORE (app-side, at relaunch)
// Read bytes from RamFS file via OP_RAMFS_OPEN+OP_RAMFS_READ+OP_RAMFS_CLOSE
let rec = AppStateRecord::from_bytes(&loaded_bytes).ok_or(ERR_INVALID)?;
rec.validate()?;                             // magic + version + checksum
rec.check_generation(expected_gen)?;          // stale rejection
// rec.data[..rec.data_len] is the app state payload
```

Quil already demonstrates the RamFS save/load pattern via `quil_save()`/`quil_load()`.
This proof provides the versioned, checksummed, generation-guarded record type
that makes the save/load contract safe against corruption and staleness.

## Contract Boundaries Preserved

- **No Linux/POSIX assumptions**: RamFS-backed, PDX-only, no file paths
- **No std/libc/threads**: pure no_std Rust
- **MPK/PKU/PKEY isolation**: sexfiles runs in PD 11, app state data is opaque bytes
- **No shared-memory redesign**: data flows through PDX message registers
- **No kernel edits**: proof uses existing RamFS backend and PDX ops
- **No sex-pdx ABI edits**: no new opcodes or slots
- **No full session manager**: AppStateRecord is a bounded 53-byte record, not a session framework

## Gate Run Commands

```bash
# Build with proof
SEXOS_APP_STATE_SAVE_RESTORE_PROOF=1 cargo build -p sexfiles --target x86_64-sex.json --release

# Run proof gate
./scripts/sexfiles_reboot_harness.sh  # or:
SEXOS_APP_STATE_SAVE_RESTORE_PROOF=1 ./scripts/master_runtime_gate.sh --probe 15 --keep-log
```
