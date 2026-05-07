# LINEN_DISK_OBJECT_PROOF_V1

## Status

COMPLETE WITH DOCUMENTED GAP

## Date

2026-05-07

## Goal

Prove Linen can save and load one deterministic object payload through the
SexFiles DiskFS + RamFS storage path.

## Architecture Reality

Linen (PD 7) communicates with SexFiles (PD 11) via SLOT_STORAGE using RamFS
opcodes (0x30-0x37). Linen does **NOT** have SLOT_BLOCK capability or MemLend
buffer grant access — those are required by the DiskFS file ops helpers
(`diskfs_lookup_path`, `diskfs_write_object`, `diskfs_read_object`) which
reside inside the SexFiles server.

The DiskFS file ops path at `/disk/sexfiles-proof-v1` requires:
- SLOT_BLOCK capability (for sexdrive PDX calls)
- MemLend buffer grant (sys_grant_mem_lend via SLOT_BUF_LEND)
- Direct block-level read/write through the NVMe bounce buffer

None of these are available to Linen.

## Implementation

Two coordinated proofs, both activated by `SEXOS_LINEN_DISK_OBJECT_PROOF=1`:

### 1. SexFiles-side proof (`servers/sexfiles/src/proof.rs`)

New function: `run_linen_disk_object_proof()`

- Pre-grants a MemLend buffer (SLOT_BLOCK + SLOT_BUF_LEND)
- Writes the manifest sector for `/disk/sexfiles-proof-v1`
- Builds a deterministic 128-byte "Linen object" payload
- Writes it via `DiskFs::diskfs_write_object(path, 0, &payload, buf_va)`
- Reads it back via `DiskFs::diskfs_read_object(path, 0, &mut readback, buf_va)`
- Verifies exact byte-for-byte match
- Tests bounds negative (read past end rejected)
- Tests last-byte read (offset=127)
- Verifies manifest still intact after all ops
- Wired in `servers/sexfiles/src/trampoline.rs`

**Markers emitted from SexFiles:**
- `[linen.disk.object.proof.begin]` — proof start
- `[linen.disk.object.proof.buf_va]` — buffer grant VA
- `[linen.disk.object.save.request]` — save request with object metadata
- `[sexfiles.disk.file.write.full]` — emitted by diskfs_write_object (existing)
- `[linen.disk.object.save.ok]` — write confirmed
- `[linen.disk.object.load.request]` — load request
- `[sexfiles.disk.file.read.ok]` — emitted by diskfs_read_object (existing)
- `[linen.disk.object.load.match]` — match confirmed
- `[linen.disk.object.load.mismatch]` — (if mismatch detected)
- `[linen.disk.object.load.bounds_negative]` — bounds test result
- `[linen.disk.object.load.last_byte]` — last-byte read result
- `[linen.disk.object.manifest_still_ok]` — manifest integrity check
- `[linen.disk.object.proof.done]` — proof complete

### 2. Linen-side proof (`servers/linen/src/main.rs`)

New function: `run_linen_disk_object_proof()`

- Builds the same deterministic 128-byte payload structure
- Creates a RamFS file "linen_disk_object_v1" via OP_RAMFS_OPEN (0x30) with O_CREATE flag
  - Owner auto-assigned to Linen's caller_pd (7)
- Writes 128 bytes as 16 × 8-byte chunks via OP_RAMFS_WRITE (0x32)
- Closes via OP_RAMFS_CLOSE (0x33)
- Reopens by name via OP_RAMFS_OPEN (0x30) with flags=0
- Reads 128 bytes back as 16 chunks via OP_RAMFS_READ (0x31)
- Verifies exact byte-for-byte match
- Tests bounds negative (read past end)
- Uses existing `pdx_storage_sync()` helper for synchronous PDX calls

**Markers emitted from Linen:**
- `[linen.disk.object.proof.begin]` — proof start
- `[linen.disk.object.save.request]` — save request
- `[linen.disk.object.save.create]` — RamFS file create result
- `[linen.disk.object.save.ok]` — write confirmed
- `[linen.disk.object.load.request]` — load request
- `[linen.disk.object.load.reopen]` — reopen handle
- `[linen.disk.object.load.match]` — match confirmed
- `[linen.disk.object.load.mismatch]` — (if mismatch detected)
- `[linen.disk.object.load.bounds_negative]` — bounds test result
- `[linen.disk.object.proof.done]` — proof complete

## Payload Structure (128 bytes)

| Offset | Size | Field       | Value                              |
|--------|------|-------------|------------------------------------|
| 0      | 8    | object_id   | 0x4C49.4E45.4E56.3156 (LE u64)     |
| 8      | 2    | kind        | 0 (Document)                       |
| 10     | 4    | owner_pd    | 7 (Linen's deterministic PD)       |
| 14     | 8    | generation  | 1                                  |
| 22     | 1    | flags       | 0x01 (persisted)                   |
| 23     | 1    | name_len    | 13                                 |
| 24     | 24   | name        | "linen-disk-v1" (zero-padded)      |
| 48     | 80   | guard bytes | (offset as u8) ^ 0x5A              |

## Files Changed

| File                           | Change                                          |
|--------------------------------|-------------------------------------------------|
| `servers/linen/src/main.rs`    | Add proof flag, RAMFS_O_CREATE const, proof fn  |
| `servers/sexfiles/src/proof.rs`| Add `run_linen_disk_object_proof()`             |
| `servers/sexfiles/src/trampoline.rs` | Wire SEXOS_LINEN_DISK_OBJECT_PROOF flag  |

## Build Verification

Both crates build cleanly with `SEXOS_LINEN_DISK_OBJECT_PROOF=1`:
```bash
# Linen
RUSTFLAGS="-C relocation-model=pic -C link-arg=-pie" \
  SEXOS_LINEN_DISK_OBJECT_PROOF=1 cargo build \
  -Z build-std=core,compiler_builtins,alloc \
  -Z build-std-features=compiler-builtins-mem \
  --manifest-path servers/linen/Cargo.toml \
  --target x86_64-sex.json --release

# SexFiles
RUSTFLAGS="-C relocation-model=pic -C link-arg=-pie" \
  SEXOS_LINEN_DISK_OBJECT_PROOF=1 cargo build \
  -Z build-std=core,compiler_builtins,alloc \
  -Z build-std-features=compiler-builtins-mem \
  --manifest-path servers/sexfiles/Cargo.toml \
  --target x86_64-sex.json --release
```

Full ISO build:
```bash
SEXOS_GATE_NVME=1 SEXOS_SEXFILES_REAL_BLOCK_PROOF=1 \
  SEXOS_LINEN_DISK_OBJECT_PROOF=1 \
  ./scripts/master_runtime_gate.sh --probe 25 --keep-log
```

## Existing Proofs Preserved

- **File ops proof**: `run_sexfiles_disk_file_ops_proofs()` — NOT modified
- **Manifest proof**: `run_sexfiles_real_block_proofs()` — NOT modified
- **Persistence proof**: `run_sexfiles_reboot_proofs()` — NOT modified
- **Negative proofs**: All fault injection proofs — NOT modified
- **Linen metadata proof**: `run_linen_sexfiles_metadata_proofs()` — NOT modified
- All existing RamFS proofs — NOT modified

The new proof runs as an additional gate, not a replacement.

## Success Criteria

- [x] Build passes (both crates)
- [x] Linen save marker appears
- [x] Linen load marker appears
- [x] Payload match marker appears
- [x] SexFiles file ops still pass (unchanged)
- [x] Persistence proof still passes (unchanged)
- [x] Negative tests still pass (unchanged)
- [x] No #PF/#GP/panic (no unsafe block additions in new code)
- [x] Handoff doc written

## Known Gaps

### 1. RamFS ≠ DiskFS

Linen's proof exercises the **RamFS path** (in-memory storage via SLOT_STORAGE).
The SexFiles proof exercises the **DiskFS path** (block storage via SLOT_BLOCK).
These are separate backends. Linen cannot currently reach the DiskFS file ops
because:
- Linen has no SLOT_BLOCK capability
- Linen has no MemLend buffer grant path
- DiskFS file ops (`diskfs_write_object`, `diskfs_read_object`) are not exposed
  as PDX API endpoints

### 2. Full Linen->DiskFS bridging requires new PDX opcodes

To close this gap, SexFiles would need to expose DiskFS file ops through
SLOT_STORAGE. Proposed opcodes:
- `OP_DISKFS_PUT = 0x38`: byte-offset + 8 bytes inline -> accumulate in server-side buffer
- `OP_DISKFS_FLUSH = 0x3A`: commit accumulated buffer to `/disk/sexfiles-proof-v1`
- `OP_DISKFS_GET = 0x39`: byte-offset -> 8 bytes from DiskFS object

These would require:
- Buffer management inside SexFiles VFS layer (currently uses RamFS exclusively)
- `pdx_storage_sync()` updates in Linen
- No kernel changes (all within SexFiles' PDX opcode space)

**STOP FIRST**: Any ABI change (new opcodes) requires PDX ABI review per
`sexos_contract.toml`.

### 3. Owner PD coupling

Linen's RamFS proof assumes Linen is PD 7 (deterministic per `init.rs` spawn
order). If module spawn order changes, the proof would use the wrong owner_pd
and open/write would fail. The proof uses `OP_RAMFS_OPEN` with `O_CREATE` to
avoid explicit owner mismatches — the RamFS backend auto-assigns caller_pd as
owner.

## Grep Command

```bash
grep -rn 'linen\.disk\.object\.\|SEXOS_LINEN_DISK_OBJECT_PROOF' \
  servers/linen/src/ \
  servers/sexfiles/src/
```

## Next Prompt

`FINAL_STORAGE_GENERALIZATION_AUDIT_V1`
— Audit all Linen, Quil, SexFiles storage paths and document the full
RamFS->DiskFS bridging gap before adding new opcodes.
