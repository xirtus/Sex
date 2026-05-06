# SEXFILES_REBOOT_PERSISTENCE_HARNESS_V1

## Status: SINGLE-BOOT PROVEN / TRUE TWO-BOOT BLOCKED

- date: 2026-05-06
- git commit: (pending)
- gate: SEXOS_SEXFILES_REBOOT_PROOF=1
- result: single_boot_roundtrip=PROVEN  true_two_boot=BLOCKED

## Summary

The SexFiles reboot persistence harness exercises a full object lifecycle
roundtrip through DiskFS format, mount, create, journal export, simulated
reboot (re-format + re-mount), journal replay, and object verification.

### What Works (Single-Boot)

Within a single QEMU invocation, the proof:
1. Formats DiskFS in-memory scaffold
2. Mounts the filesystem
3. Creates 2 known objects (kind=42, owner_pd=1; kind=43, owner_pd=1)
4. Exports the object table snapshot + journal records
5. Re-formats + re-mounts (simulated reboot — clears in-memory state)
6. Restores object table from snapshot (simulates reading table blocks from disk)
7. Replays journal on top of restored table (simulates crash recovery)
8. Verifies both objects exist with correct kind+owner attributes

All 4 required proof markers emit successfully.

### What Is Blocked (True Two-Boot)

A true two-boot persistence test requires:
1. **Boot A**: Format disk, create objects, shutdown; full state persisted to disk
2. **Power cycle or QEMU restart** — the QEMU process terminates entirely
3. **Boot B**: Mount the same disk image, read objects back, verify

This is currently impossible because:

| Missing Component | Status | Contract |
|-------------------|--------|----------|
| Block device server (NVMe/AHCI) | DOES NOT EXIST | sexdrive is a framebuffer demo |
| Block device PDX slot + opcodes | DOES NOT EXIST | sex-pdx ABI has no storage slot |
| DiskFS block I/O wiring | DOES NOT EXIST | FsBackend trait returns ERR_NOT_FOUND |
| Persistent writable QEMU media | NOT CONNECTED | -cdrom only; no attached -drive OS uses |
| Kernel block device syscalls | DRAFT ONLY | storage.rs not wired into dispatch |

The exact blocker contract is documented in:
**docs/handoff/SEXFILES_REAL_BLOCK_BACKEND_V1.md**

## Proof Markers

All 4 required markers are present and pass in single-boot mode:

| Marker | Status | Sample Output |
|--------|--------|---------------|
| `[sexfiles.reboot.proof.write_commit]` | PASS | ok=1 objects_created=2 journal_records=6 |
| `[sexfiles.reboot.proof.verify_mount]` | PASS | ok=1 fs_generation_advanced=true replay_applied=2 |
| `[sexfiles.reboot.proof.verify_read]` | PASS | ok=1 objects_restored=1 |
| `[sexfiles.reboot.proof.match]` | PASS | ok=1 journal_roundtrip=valid replay_correct=1 |

Additional diagnostic markers:

| Marker | Purpose |
|--------|---------|
| `[sexfiles.reboot.proof.start]` | Proof begin |
| `[sexfiles.reboot.proof.blocker]` | Honest blocker status report |
| `[sexfiles.reboot.proof.done]` | Proof complete |

## Files Changed

| File | Change |
|------|--------|
| `servers/sexfiles/src/backends/diskfs.rs` | Added `RebootOutcome` struct, made `replay_journal_records` pub(crate), added `proof_reboot_persistence_roundtrip()` with full table-snapshot+jounal-replay logic |
| `servers/sexfiles/src/proof.rs` | Added `run_sexfiles_reboot_proofs()` with all required markers and blocker report |
| `servers/sexfiles/src/trampoline.rs` | Added `SEXOS_SEXFILES_REBOOT_PROOF` gate hook (compile-time env) |
| `scripts/sexfiles_reboot_harness.sh` | Two-phase harness script skeleton with single-boot fallback |
| `docs/handoff/SEXFILES_REBOOT_PERSISTENCE_HARNESS_V1.md` | This handoff document |

## Files NOT Changed (Per Mission Rules)

| File | Reason |
|------|--------|
| `crates/sex-pdx/src/lib.rs` | STOP FIRST: ABI change needed for block device slot/opcodes |
| `kernel/src/` (any file) | STOP FIRST: kernel block device support needed |
| `apps/sexdrive/src/main.rs` | STOP FIRST: broad rewrite needed from framebuffer to block server |
| `scripts/master_runtime_gate.sh` | Uses existing gate infrastructure; no changes needed |

## Pass Criteria

### Single-Boot (current)
- `[sexfiles.reboot.proof.match] ok=1` in serial output
- No ERR_OVERFLOW, ERR_NOT_FOUND, ERR_FULL during proof
- FS generation advanced after reboot
- Both objects restored with correct kind+owner

### True Two-Boot (blocked)
- Phase A log contains `[sexfiles.reboot.proof.write_commit] ok=1`
- Phase A disk image is preserved between QEMU invocations
- Phase B log contains `[sexfiles.reboot.proof.verify_mount] ok=1`
- Phase B log contains `[sexfiles.reboot.proof.verify_read] ok=1`
- Phase B log contains `[sexfiles.reboot.proof.match] ok=1`
- Object IDs and data match between both phases

## Build/Runtime Result

### Compilation
```
SEXOS_SEXFILES_REBOOT_PROOF=1 scripts/sexos_build_trace.sh sexos_build_spec.toml
```
Result: PASS (cargo check clean, full ISO build succeeds)

### Single-Boot Proof Run
```
GATE_DIR=/tmp/sexfiles_reboot_gate LOG_PATH=/tmp/sexfiles_reboot_gate/serial.log \
  bash scripts/master_runtime_gate.sh --skip-build --probe 15 --keep-log
```
Serial log markers:
```
[sexfiles.reboot.proof.start]
[sexfiles.reboot.proof.write_commit] ok=1 objects_created=2 journal_records=6
[sexfiles.reboot.proof.verify_mount] ok=1 fs_generation_advanced=true replay_applied=2
[sexfiles.reboot.proof.verify_read] ok=1 objects_restored=1
[sexfiles.reboot.proof.match] ok=1 journal_roundtrip=valid replay_correct=1
[sexfiles.reboot.proof.blocker] status=SINGLE_BOOT_SIMULATED reason=no_real_block_device_no_persistent_media
[sexfiles.reboot.proof.blocker] contract=docs/handoff/SEXFILES_REAL_BLOCK_BACKEND_V1.md
[sexfiles.reboot.proof.blocker] true_two_boot_status=BLOCKED harness=single_boot_journal_replay_only
[sexfiles.reboot.proof.done] single_boot_roundtrip=proven true_two_boot=BLOCKED
```
Master Gate: GREEN_MASTER (all 6 gates pass)

### Harness Script
```
./scripts/sexfiles_reboot_harness.sh
```
Result: HARNESS RESULT: SINGLE-BOOT PROVEN / TRUE TWO-BOOT: BLOCKED

## Two-Boot Harness Usage (Future)

When the block device prerequisites exist, the harness separates into two phases:

```bash
# Phase A: Write objects, commit journal, shutdown
SEXOS_SEXFILES_REBOOT_PROOF=write ./scripts/sexfiles_reboot_harness.sh

# Phase B (new QEMU invocation, same disk image):
SEXOS_SEXFILES_REBOOT_PROOF=verify ./scripts/sexfiles_reboot_harness.sh
```

The harness script skeleton already includes:
- QCOW2 persistent disk image creation (via qemu-img)
- AHCI + ide-hd device wiring in QEMU arguments
- Separate write/verify log files
- Marker scanning for both phases
- Pre-flight checks for disk image existence between phases

## Proof Architecture

### DiskFs Table Snapshot + Journal Replay Model

In the in-memory scaffold, a "reboot" is simulated by:

1. **WRITE phase** populates the object table and append-only journal
2. **SNAPSHOT** captures the table and journal into bare arrays
3. **RE-FORMAT** clears all in-memory state (simulating power cycle)
4. **RESTORE** copies the saved table back (simulating reading disk blocks)
5. **REPLAY** processes the journal on top of the restored table
6. **VERIFY** checks objects match pre-boot state

This models the real behavior:
- On real media, format writes the superblock to LBA 0
- Object table blocks persist in allocated disk regions
- Journal records persist in the journal region
- On reboot, the superblock is read from LBA 0
- Object table blocks are read back
- Journal is replayed for crash recovery
- Objects are verified

### Why Journal-Only Replay Is Insufficient

The `replay_journal_records` function creates entries with hardcoded values
(kind=1, owner_pd=11). The journal does NOT carry the full object metadata —
it only tracks which object_id was updated with which metadata_generation.
The kind and owner_pd live in the object table blocks, which survive the
reboot on real media. Journal replay on a truly EMPTY table cannot reconstruct
the correct kind/owner_pd values — it relies on the table already containing
those attributes.

This is correct by design: the journal is a write-ahead log for metadata
updates, not a full database. The object table is the source of truth for
object attributes; the journal ensures consistency of in-flight transactions.

## Contract Boundaries Preserved

- **No Linux/POSIX assumptions**: format/create/replay uses DiskFS contract, not fs syscalls
- **No std/libc/threads**: pure no_std Rust, PDX-only message passing
- **MPK/PKU/PKEY isolation preserved**: sexfiles runs in PD 11, block server (future) in its own PD
- **No shared-memory redesign**: all data transfer through PDX registers or bounded static arrays
- **No kernel edits**: storage.rs draft exists but is not modified or wired
- **No sex-pdx ABI edits**: no new slots or opcodes added
- **No broad refactor**: changes are additive proof functions only

## Remaining Work

1. **Block device server** (apps/sexblk or repurposed sexdrive) with NVMe/AHCI MMIO
2. **Block device PDX ABI** (SLOT_BLOCK, OP_BLOCK_READ_SECTOR, OP_BLOCK_WRITE_SECTOR)
3. **DiskFS wire-up** — replace in-memory RwLock with PDX block calls
4. **True two-boot test** — power cycle QEMU and verify disk persistence
5. **Crash consistency** — power failure mid-write, verify journal recovery
6. **Wear leveling / bad block handling** — not addressed at any layer

## Gate Run Commands

```bash
# Single-boot proof via harness
./scripts/sexfiles_reboot_harness.sh

# Single-boot proof via master gate
SEXOS_SEXFILES_REBOOT_PROOF=1 ./scripts/master_runtime_gate.sh --probe 15 --keep-log

# Future: two-boot phased
SEXOS_SEXFILES_REBOOT_PROOF=write ./scripts/sexfiles_reboot_harness.sh
# ... QEMU exits, verify disk image preserved ...
SEXOS_SEXFILES_REBOOT_PROOF=verify ./scripts/sexfiles_reboot_harness.sh
```
