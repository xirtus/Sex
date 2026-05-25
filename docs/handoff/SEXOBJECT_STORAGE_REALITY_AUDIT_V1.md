# SEXOBJECT_STORAGE_REALITY_AUDIT_V1

**Date**: 2026-05-25
**Mission**: Audit current SexObject / Linen / SexFiles / DiskFS / SexDrive storage reality before SexFS v0 object-store format.
**Status**: AUDIT ONLY — no implementation changes.

---

## A) Outcome: PASS

All 10 audit questions answered with grep-backed evidence. The current storage tier is an
**in-memory bounded scaffold** with full contract shapes, but **no real persistent block I/O backing**.
Everything that claims persistence is honestly deferred; nothing overclaims.

---

## B) Current Storage Truth

### B1. Architecture Summary

```
Linen (PD 7)
  pdx_storage_sync(opcode, arg0, arg1, arg2)
  --> pdx_call(SLOT_STORAGE=1, ...)  <-- NO SLOT_BLOCK capability

  Uses RamFS opcodes (0x30-0x37,0x3D,0x3F)
  Uses DiskFS bridge opcodes (0x38-0x3E)
  |
  v
SexFiles (PD 11)
  SLOT_STORAGE handler --> vfs::pdx_ramfs_dispatch()
    - RamFS opcodes --> backend: &RAMFS (in-memory)
    - DiskFS bridge --> handle_diskfs_*() functions
        --> DiskFs::diskfs_write_object/read_object/...
        --> DiskFs::diskfs_block_write/read(BLOCK_*)
        --> pdx_call(SLOT_BLOCK=15, BLOCK_*, ...)
  |
  v
SexDrive (framebuffer demo only)
  SLOT_BLOCK handler --> stub/placeholder
  NO real NVMe/AHCI backend wired.
  BLOCK_READ returns unconsumed data (no real I/O).
```

### B2. DiskFS — In-Memory Scaffold

File: `servers/sexfiles/src/backends/diskfs.rs` (2722 lines)

BLOCKER comment at line 228-241:
> "DiskFs: bounded in-memory mock scaffold for V1 on-disk format lock.
> BLOCKER: No real block I/O path is wired yet in sexfiles->sexdrive.
> The system lacks: a block device server (sexdrive is a framebuffer demo),
> block device PDX opcodes/slots, block device kernel syscalls,
> and any NVMe/AHCI driver infrastructure."

**Scaffold evidence**:
- diskfs.rs:243: `pub struct DiskFs;` — no fields, no instance state; everything lives in `static DISKFS_STATE: RwLock<DiskFsState>` (line 245)
- diskfs.rs:2686-2718: `impl FsBackend for DiskFs` — **all methods return `Err(messages::ERR_NOT_FOUND)`** — the DiskFS backend is NOT wired as an FsBackend; the VFS dispatch at vfs.rs:377 always routes to `&RAMFS`

### B3. What Actually Exists

| Component | Real? | Location | Notes |
|-----------|-------|----------|-------|
| **Superblock** (`SexfilesSuperblock`) | scaffold | diskfs.rs:64-74 | magic=0x3156_5345_4C49_4653, version_major=1, block_size=4096 |
| **Object table** (`table: [SexfilesObjectEntry; 16]`) | scaffold | diskfs.rs:156 | 16-entry bounded array, in-memory only |
| **Object entry** (`SexfilesObjectEntry`) | scaffold | diskfs.rs:77-87 | object_id, kind, owner_pd, rights_generation, object_size_bytes, first_block, metadata_generation, checksum, in_use |
| **Free-space map** (`extent_bitmap: [u64; 16]`) | scaffold | diskfs.rs:162 | 1024 blocks x 4096 bytes = 4 MiB, first-fit allocator |
| **Journal** (`journal: [JournalRecord; 64]`) | scaffold | diskfs.rs:157-158 | WAL journal with TxBegin/ObjectMetadataUpdate/TxCommit |
| **Checkpoints** (`checkpoints: [SexfilesCheckpoint; 4]`) | scaffold | diskfs.rs:167-168 | Generational snapshots of the full object table |
| **Manifest** (LBA 2046 sector) | real block | diskfs.rs:29,33 | V1 single-entry + V2 multi-entry (3 paths) |
| **Mount validation** (`mount()`) | scaffold | diskfs.rs:1495-1510 | Checks magic, version, block_size, entry_count, checksum |
| **Allocate/free blocks** | scaffold | diskfs.rs:852-916 | allocate_blocks(count), free_blocks(first, count) |
| **Create/stat object** | scaffold | diskfs.rs:1502-1545 | create_object_entry(kind, owner) -> object_id |
| **Journal replay** | scaffold | diskfs.rs:1115-1231 | replay_journal_records() -- committed-only, gen-ordered |
| **Checkpoint create/restore** | scaffold | diskfs.rs:1591-1700 | create_checkpoint(), restore_checkpoint(cp) |
| **DiskFS file ops** (write/read_object) | real PDX route | diskfs.rs:2223-2460 | Write/read via manifest->BLOCK_WRITE/READ at LBAs |
| **Block I/O bridge** | real PDX calls | diskfs.rs:298-375 | Uses pdx_call(SLOT_BLOCK, BLOCK_*, ...) -- real PDX, but SexDrive is stub |
| **Manifest write/read sector** | real block route | diskfs.rs:518-561 | Uses sys_grant_mem_lend -> diskfs_block_write/read |
| **Reboot roundtrip proof** | scaffold only | diskfs.rs:1336-1484 | Saves table+journal to local vars, re-creates state, replays -- NO actual disk persistence |

### B4. What Survives Reboot Today: NOTHING (Honestly Deferred)

**Linen classification** (main.rs:2992-2995):
```
[linen.reboot_restore.skip] reason=no_ioq_ready_model_only_dispatch_deferred
  model_only=1 durable=0
[linen.reboot_restore.truth] direct_save_load=proven reboot_restore=deferred ok=1
[linen.reboot_restore.done] classification=honest_skip powerloss=0 journal=0 ok=1
```

**Linen Object UX** (main.rs:3037):
```
[linen.object_ux.deferred] reboot_restore=1 durable=0 powerloss=0 journal=0 ok=1
```

**Reboot roundtrip proof** (diskfs.rs:1336-1484) is a contract proof, not a persistence proof:
- Saves `saved_table` and `saved_journal` to local stack variables
- Re-creates a fresh DiskFs state (simulating "reboot")
- Copies state back from local variables (NOT from disk)

**DiskFs FsBackend impl** (diskfs.rs:2686-2718): all methods return ERR_NOT_FOUND -- not wired.

### B5. Current DiskFS Path: Fixed-Block/Fixed-Object Only

3 hardcoded paths (diskfs.rs:46-52):
```
path_id 0 -> /disk/sexfiles-proof-v1  (LBA 2038-2045)
path_id 1 -> /disk/linen-object-v1    (LBA 2030-2037)
path_id 2 -> /disk/quil-object-v1     (LBA 2022-2029)
```

- Fixed 4096-byte objects (DISKFS_OBJECT_SLOT_SIZE = 4096)
- 8 sectors each (DISKFS_OBJECT_SLOT_SECTORS = 8)
- No dynamic path creation, no directory hierarchy, no variable-size objects

---

## C) Existing Reusable Pieces for SexFS v0

### C1. Ready for Direct Reuse (Contract-Proven Shapes)

| Piece | File:Line | Lines | Reuse Strategy |
|-------|-----------|-------|----------------|
| **SexfilesSuperblock** struct | diskfs.rs:64-74 | 11 | Keep. Add pad to 512-byte sector. Add on-disk write/read. |
| **SexfilesObjectEntry** struct | diskfs.rs:77-87 | 11 | Keep. Add on-disk serialization. V1 layout is flat. |
| **SexfilesCheckpoint** struct | diskfs.rs:110-117 | 9 | Keep. Already has magic+checksum. |
| **JournalRecord** struct | diskfs.rs:120-128 | 10 | Keep. Already has kind/tx_id/generation/checksum. |
| **JournalRecordKind** enum | diskfs.rs:91-95 | 5 | Keep. |
| **Extent bitmap** (free-space map) | diskfs.rs:162 | 1 | Keep. 1024 blocks, first-fit. Persist to LBA of freemap region. |
| **Mount validation** | diskfs.rs:1495-1510 | 16 | Keep. Add real block read from LBA 0 (superblock). |
| **Checksum functions** | diskfs.rs:616-660 | ~80 | Keep. XOR-based for superblock/entry/journal/checkpoint. |
| **Journal replay** | diskfs.rs:1115-1231 | ~120 | Keep. Already handles committed-only, gen-ordered replay. |
| **Checkpoint create/restore** | diskfs.rs:1591-1700 | ~110 | Keep. Already handles magic+checksum validation and table restore. |
| **Manifest format** (V1+V2) | diskfs.rs:429-480,2534-2677 | ~240 | Keep. Already has magic, version, entry_count, entry parsing. |
| **diskfs_lookup_path** | diskfs.rs:2171-2221 | ~50 | Keep. Already reads manifest from block device. |
| **diskfs_write/read_object** | diskfs.rs:2223-2460 | ~240 | Keep. Read-modify-write per sector. Needs real block device backing only. |
| **diskfs_block_write/read** | diskfs.rs:298-375 | ~80 | Keep. Already uses pdx_call(SLOT_BLOCK, BLOCK_*, ...). Blocked only by sexdrive stub. |
| **Manifest ensure** (V1->V2 bootstrap) | diskfs.rs:2534-2677 | ~145 | Keep. Already writes/reads/verifies manifest to/from block device. |

### C2. SexObject Model Crate — Ready

`crates/sex-object-model/src/lib.rs` (191 lines):
- `SexObjectHeader` -- 80-byte logical header
- `SexObjectRef` -- 16-byte cross-PD reference
- `SexObjectKind` -- bounded enum (12 kinds, fits u16)
- Flags: FLAG_TOMBSTONED, FLAG_SEALED, FLAG_REDACTED, FLAG_MIGRATING

`servers/sexfiles/src/sexobject.rs` (70 lines):
- `sexobject_header_from_entry()` -- pure adapter from SexfilesObjectEntry -> SexObjectHeader
- content_generation is V1 proxy using metadata_generation until M2 disk extension

### C3. Opcode/Routing Infrastructure — Ready

**RamFS opcodes** (servers/sexfiles/src/messages.rs):
```
0x30 OP_RAMFS_OPEN       0x31 OP_RAMFS_READ       0x32 OP_RAMFS_WRITE
0x33 OP_RAMFS_CLOSE      0x34 OP_RAMFS_LIST       0x35 OP_RAMFS_STAT
0x36 OP_RAMFS_CREATE_OWNER 0x37 OP_RAMFS_OBJECT_ID
0x3D OP_RAMFS_READNAME   0x3F OP_RAMFS_STATUS
```

**DiskFS bridge opcodes** (servers/sexfiles/src/messages.rs):
```
0x38 OP_DISKFS_WRITE      0x39 OP_DISKFS_READ       0x3A OP_DISKFS_FLUSH
0x3B OP_DISKFS_STAT       0x3C OP_DISKFS_MANIFEST_HASH
0x3E OP_DISKFS_SELECT
```

**VFS dispatch** (servers/sexfiles/src/vfs.rs:525-557):
All DiskFS bridge opcodes routed through handle_diskfs_*() -> DiskFs::* -> SLOT_BLOCK

### C4. Proof Infrastructure — Ready

`servers/sexfiles/src/proof.rs` (3601 lines, ~12 proof functions):
```
run_diskfs_object_table_proofs()         -- superblock + object table contract
run_linen_disk_object_proof()            -- Linen->DiskFS bridge proof
run_diskfs_multi_object_proofs()         -- V2 manifest + multi-object
run_diskfs100_ap2_proof()                -- format/init contract
run_diskfs_bridge_strict_proof_v1()      -- PDX route validation
run_diskfs_negative_bounds_auth_proof()  -- all rejection cases
run_diskfs100_ap4_write_proof()          -- write contract
run_diskfs100_ap4_read_proof()           -- read contract
```

---

## D) Missing Pieces for SexObject/SexFS v0

### D1. CRITICAL -- Real Block Device Backend (THE Blocker)

| Missing | Why | How to Add |
|---------|-----|------------|
| NVMe/AHCI driver in sexdrive | All block I/O goes through sexdrive's SLOT_BLOCK handler | Write NVMe admin init + SQ/CQ + PRP in sexdrive |
| SexDrive -> actual NVMe PCI device | Currently framebuffer demo only | Map PCI BAR, set up admin queue, identify namespace |
| BLOCK_READ -> real DMA read | Currently returns unconsumed/uninitialized data | Set up PRP list, submit read command, wait for completion |
| BLOCK_WRITE -> real DMA write | Currently returns status 0 but writes nothing | Same as read but with write command |
| BLOCK_SYNC -> real NVMe flush | Currently stub | Submit flush command, wait for completion |
| MemLend DMA buffer -> physical pages | Buffer VA granted but not backed by actual DMA pages | Kernel maps MemLend pages to NVMe PRP |

Evidence: diskfs.rs:232-233 ("sexdrive is a framebuffer demo"),
proof.rs:1072-1076/1159-1161 ("blocker=PAYLOAD_HANDOFF_MISSING ... blocker=REAL_DEVICE_BACKEND_MISSING")

### D2. HIGH -- Persistent Superblock Read/Write

| Missing | Why | How to Add |
|---------|-----|------------|
| Read superblock from LBA 0 on mount | mount() reads from in-memory st.superblock | Add diskfs_block_read(0, 512, ...) at LBA 0 |
| Write superblock to LBA 0 on format | format_init_empty() writes memory only | Add diskfs_block_write(0, 512, ...) |
| Superblock sector layout | Current struct has no on-disk padding | Add 512-byte aligned struct with reserved/pad bytes |

### D3. HIGH -- Persistent Object Table Read/Write

| Missing | Why | How to Add |
|---------|-----|------------|
| Read object table from disk on mount | Current mount reads from memory | Read blocks starting at superblock.object_table_start_block |
| Write object table to disk on commit | create_object_entry() writes memory only | After modifying table, write affected blocks to disk |
| Object table sector layout | 16 entries need serialization | Serialize entries to fixed-size records in sectors |

### D4. HIGH -- Persistent Free-Space Map Read/Write

| Missing | Why | How to Add |
|---------|-----|------------|
| Read extent bitmap from disk on mount | Current bitmap is memory-only | Read bitmap blocks from reserved LBA region |
| Write extent bitmap on alloc/free | Current bitmap ops are memory-only | Journal extent changes, write bitmap blocks |

### D5. MEDIUM -- Reboot Survival (Real, not Scaffold)

| Missing | Why | How to Add |
|---------|-----|------------|
| Object recovery from disk after crash | proof_reboot_persistence_roundtrip uses local vars | Mount -> read superblock -> read object table -> replay journal |
| Journal persistence | Journal is in-memory only | Write journal records to LBA region on each append |
| Checkpoint persistence | Checkpoints are in-memory only | Write checkpoint to reserved LBA region |
| Two-boot proof | Current "reboot" is simulated within one process | Real two-boot requires writing on boot 1, power-cycling QEMU, reading on boot 2 |

### D6. MEDIUM -- Dynamic Object Store Features

| Missing | Why | How to Add |
|---------|-----|------------|
| Dynamic path/lookup | Current manifest has 3 hardcoded paths | Extend manifest to support dynamic entries (name->LBA mapping) |
| Variable-size objects | Current objects are fixed 4096 bytes | Allocate variable number of blocks, store len in entry |
| Object deletion | No delete path exists | Add tombstone flag to entry, free blocks, update manifest |
| Directory hierarchy | No directories exist | MUST remain deferred -- not in scope for v0 |
| Rename | No rename path exists | MUST remain deferred -- not in scope for v0 |

### D7. LOW -- SexObject-Specific Gaps

| Missing | Why | How to Add |
|---------|-----|------------|
| content_generation field | Currently proxy (uses metadata_generation) | Add real content_generation to SexfilesObjectEntry (M2) |
| SexObjectHeader -> disk roundtrip | sexobject_header_from_entry() is read-only | Add sexobject_header_to_entry() for write-back |
| Rights revocation bridge (Collar->SexFiles) | collar_rights_generation() is stub | Wire new PDX opcode for Collar->SexFiles rights_generation bump |

---

## E) Non-Claims (Current Tier Honest Limitations)

From run_linen_object_ux_current_tier_proof() (main.rs:3037-3040):

```
linen_presents=honest_bounded_fixed_object_ux
overclaims=0
proves=save_load+bounds_auth
defers=reboot_restore
denies=posix+filesystem+durability
ok=1
```

**Explicitly NOT claimed and must remain non-claimed for SexFS v0:**
- POSIX semantics
- General filesystem
- Directories
- Rename
- Delete
- Durability
- Powerloss safety
- Journaling guarantees

---

## F) Recommended Next Autopilot Sequence

```
Phase 1: SEXFS_V0_SUPERBLOCK_WIRED
  -- Write superblock to LBA 0 on format
  -- Read superblock from LBA 0 on mount
  -- Verify magic + checksum after real read
  -- Proof: format -> write -> read -> verify

Phase 2: SEXFS_V0_OBJECT_TABLE_WIRED
  -- Write object table blocks on commit
  -- Read object table blocks on mount
  -- Journal replay after table read
  -- Proof: create objects -> write -> reboot (new QEMU) -> read -> verify

Phase 3: SEXFS_V0_FREEMAP_WIRED
  -- Write extent bitmap to reserved LBA region
  -- Read extent bitmap on mount
  -- Proof: allocate blocks -> write -> reboot -> verify bitmap

Phase 4: SEXFS_V0_MANIFEST_DYNAMIC
  -- Extend manifest to support dynamic entries (beyond 3 hardcoded)
  -- Variable-length object allocation
  -- Proof: create object -> manifest entry written -> read object

Phase 5: SEXFS_V0_REBOOT_PROOF_REAL
  -- Two-boot proof with real QEMU power cycle
  -- Boot 1: create objects, write, flush
  -- Boot 2: mount, read objects, verify
  -- Proof gate in daily_driver_master_gate.sh

Phase 6: SEXFS_V0_LIVE_USB_READY
  -- SexFS v0 on real USB block device
  -- Boot from USB, create objects, unmount, reboot, re-mount, verify
```

---

## G) STOP FIRST Risks

1. **DO NOT wire real block I/O in sexdrive** without first proving NVMe admin init works in isolation -- a broken NVMe driver will hang the kernel.

2. **DO NOT edit `crates/sex-pdx`** without explicit STOP FIRST review -- the opcode namespace is shared across all servers.

3. **DO NOT add Linen->SexDrive direct calls** -- Linen must use only SLOT_STORAGE->SexFiles->DiskFS->SLOT_BLOCK->SexDrive.

4. **DO NOT change the SexObject ABI** (SexObjectHeader layout) without updating all consumers -- the 80-byte header is a cross-PD contract.

5. **DO NOT remove the RamFS backend** -- it remains the primary VFS backend until DiskFS impl FsBackend is real.

6. **DO NOT remove the honest deferred markers** -- they are proof that the system does not overclaim.

7. **DO NOT add POSIX semantics** -- directories, rename, delete, symlinks, permissions hierarchy must remain deferred.

8. **DO NOT claim durability** -- even after real block I/O, durability/powerloss safety require journal-to-disk persistence, a separate phase.

---

## H) Exact Audit Evidence (grep References)

### H1. DiskFS is an in-memory scaffold

```
diskfs.rs:228-241: "bounded in-memory mock scaffold ... No real block I/O path"
diskfs.rs:2686-2718: impl FsBackend for DiskFs -- ALL methods return ERR_NOT_FOUND
vfs.rs:377: let backend: &dyn FsBackend = &RAMFS;
```

### H2. Superblock exists but memory-only

```
diskfs.rs:64-74:  pub struct SexfilesSuperblock { magic: u64, ... }
diskfs.rs:19:     const DISKFS_MAGIC = 0x3156_5345_4C49_4653
diskfs.rs:1495-1510: fn mount() reads from st.superblock (memory), not disk
diskfs.rs:1488-1493: fn format_init_empty() writes to memory, not disk
```

### H3. Object table exists but memory-only

```
diskfs.rs:12:  pub const DISKFS_MAX_OBJECTS = 16
diskfs.rs:77-87: pub struct SexfilesObjectEntry
diskfs.rs:156: table: [SexfilesObjectEntry; DISKFS_MAX_OBJECTS]
diskfs.rs:1502-1545: fn create_object_entry() modifies st.table (memory)
```

### H4. Free-space map exists but memory-only

```
diskfs.rs:25:  pub const DISKFS_EXTENT_BLOCK_COUNT = 1024
diskfs.rs:26:  pub const DISKFS_EXTENT_BITMAP_WORDS = 16
diskfs.rs:162: extent_bitmap: [u64; DISKFS_EXTENT_BITMAP_WORDS]
diskfs.rs:852-875: fn allocate_blocks() modifies extent_bitmap (memory)
```

### H5. Mount validation exists (scaffold)

```
diskfs.rs:1495-1510: fn mount() checks magic, version, block_size, entry_count, checksum
proof.rs:73:  let sb = disk.mount().expect("[diskfs.proof] mount failed");
```

### H6. Journal + checkpoint exist (scaffold)

```
diskfs.rs:13:  pub const DISKFS_JOURNAL_CAPACITY = 64
diskfs.rs:18:  pub const DISKFS_MAX_CHECKPOINTS = 4
diskfs.rs:20:  const DISKFS_CHECKPOINT_MAGIC = 0x4348_4B50_4E54_5631
diskfs.rs:157-158: journal + journal_len
diskfs.rs:167-168: checkpoints + next_checkpoint_generation
```

### H7. Reboot does NOT survive (honestly deferred)

```
linen/main.rs:2993: "[linen.reboot_restore.skip] reason=no_ioq_ready_model_only_dispatch_deferred"
linen/main.rs:3037: "[linen.object_ux.deferred] reboot_restore=1 durable=0 powerloss=0 journal=0"
diskfs.rs:1336-1484: proof_reboot_persistence_roundtrip saves to local vars, not disk
```

### H8. Block I/O route exists but sexdrive is stub

```
diskfs.rs:261: pub fn diskfs_block_call(opcode, arg0, arg1, arg2) -- pdx_call(SLOT_BLOCK, ...)
proof.rs:1072-1076: "typed BLOCK_READ via SLOT_BLOCK=15"
proof.rs:1159-1161: "blocker=PAYLOAD_HANDOFF_MISSING ... blocker=REAL_DEVICE_BACKEND_MISSING"
```

### H9. Manifest is real (block-backed)

```
diskfs.rs:29:  pub const DISKFS_MANIFEST_LBA = 2046
diskfs.rs:33:  pub const DISKFS_MANIFEST_MAGIC = 0x3156_4D4B_5349_4453
diskfs.rs:518-533: proof_manifest_write_sector -- grants MemLend, writes via diskfs_block_write
diskfs.rs:540-550: proof_manifest_read_sector  -- grants MemLend, reads via diskfs_block_read
```

### H10. SexObject model exists

```
crates/sex-object-model/src/lib.rs:129: pub struct SexObjectHeader -- 80 bytes
crates/sex-object-model/src/lib.rs:83:  pub struct SexObjectRef -- 16 bytes
servers/sexfiles/src/sexobject.rs:23: pub fn sexobject_header_from_entry(entry) -> SexObjectHeader
```

### H11. Current fixed-object paths

```
diskfs.rs:46: DISKFS_OBJECT_PATH_SEXFILES = b"/disk/sexfiles-proof-v1"
diskfs.rs:47: DISKFS_OBJECT_PATH_LINEN    = b"/disk/linen-object-v1"
diskfs.rs:48: DISKFS_OBJECT_PATH_QUIL     = b"/disk/quil-object-v1"
diskfs.rs:49-52: SLOT_LINEN_LBA / QUIL_LBA / SLOT_SECTORS / SLOT_SIZE
```

### H12. Opcodes and routes

```
messages.rs:10-125: All opcodes (0x30-0x3F) with route documentation
vfs.rs:525-557: DiskFS bridge dispatch (0x38-0x3E)
vfs.rs:377-475: RamFS dispatch (0x30-0x37, 0x3D, 0x3F)
sex-pdx/src/lib.rs:388: SLOT_STORAGE = 1
sex-pdx/src/lib.rs:401: SLOT_BLOCK   = 15
```

---

*End of audit. No files changed. Commit: this audit doc only.*
