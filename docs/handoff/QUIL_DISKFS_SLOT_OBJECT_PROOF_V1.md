# QUIL_DISKFS_SLOT_OBJECT_PROOF_V1

## Date
2026-05-07

## Status
STOP FIRST — SELECT 0x3D not yet implemented. Quil bridge-capable but slot blocked.

## 1. Canonical Quil Reality

| Attribute | Value |
|-----------|-------|
| Crate | `servers/quil/` |
| PD | 9 (domain_id=9 per init.rs spawn order) |
| Capabilities | SLOT_DISPLAY, SLOT_STORAGE |
| RamFS save/load | FULL (quil_save, quil_load via OP_RAMFS_OPEN/WRITE/READ/CLOSE) |
| Sync PDX wrapper | pdx_storage_call() — same pattern as Linen's pdx_storage_sync() |
| Persistence proof | SEXFILES_QUIL_REBOOT_PERSISTENCE_PROOF_V1 (cfg-gated) |
| Storage cap probe | SEXOS_STORAGE_CAP_PROOF (env-gated, probes SLOT_STORAGE) |
| UI | Palette with SAVE_DOCUMENT (cmd=2) and LOAD_DOCUMENT (cmd=3) |
| DiskFS bridge access | NOT YET — no DISKFS opcode constants in Quil |

## 2. What Quil CAN Do Right Now

Quil has everything needed to use the DiskFS bridge:
- `SLOT_STORAGE` capability (already granted by init.rs)
- `pdx_storage_call()` synchronous wrapper (identical pattern to Linen)
- `QUIL_BUFFER` with content to persist (up to 512 bytes)
- Name packing helpers (`pack_name`)

The missing pieces (trivial to add):
- DiskFS bridge opcode constants (0x38-0x3C)
- A proof function that calls WRITE/READ via these opcodes

## 3. What Blocks path_id=2

Quil's designated DiskFS slot is:

| Field | Value |
|-------|-------|
| path_id | 2 |
| Path | `/disk/quil-object-v1` |
| Hash | `0xaaf5c55ad6c063b5` |
| LBA | 2022-2029 (8 sectors, 4096 bytes) |
| Flags | 0x3 (READ\|WRITE) |

To select this slot, Quil needs:

```
OP_DISKFS_SELECT = 0x3D
arg0 = 2  (path_id for Quil object)
```

**SELECT 0x3D is planned but not yet implemented** (see
`SEXFILES_DISK_MULTI_OBJECT_MANIFEST_PLAN_V1.md`). Until SELECT exists,
the bridge operates on path_id=0 (/disk/sexfiles-proof-v1) by default.
Quil using path_id=0 would collide with Linen's usage of the same slot.

## 4. Path Forward (Two Options)

### Option A: Implement SELECT 0x3D First (Preferred)

Complete `SEXFILES_DISK_MULTI_OBJECT_MANIFEST_IMPL_V1`, then add Quil proof:

```
1. Implement SELECT 0x3D + V2 manifest per the multi-object plan
2. Add DISKFS opcode constants to Quil (0x38-0x3D)
3. Add quil_diskfs_proof() function:
   - pdx_storage_call(0x3D, 2, 0, 0)  // SELECT path_id=2
   - Write QUIL_BUFFER content via OP_DISKFS_WRITE (0x38)
   - Read back via OP_DISKFS_READ (0x39)
   - Verify match
4. Gate behind SEXOS_QUIL_DISKFS_PROOF env var
5. Markers: quil.diskfs.slot.*
```

### Option B: Prove on path_id=0 Now (Temporary)

Quil can use the existing bridge (default path_id=0) immediately,
demonstrating bridge capability. Markers would note "slot=0 not
path_id=2 — awaiting SELECT implementation."

```
1. Add DISKFS opcode constants to Quil (0x38-0x3C)
2. Write QUIL_BUFFER to path_id=0 via OP_DISKFS_WRITE
3. Read back via OP_DISKFS_READ
4. Verify match
5. Marker: quil.diskfs.slot.proof slot=0 note=awaiting_select_for_path_id_2
```

**Risk**: Collides with Linen's usage of path_id=0. Both writing to the
same slot would corrupt each other's data. Only viable if Linen bridge
proof is NOT running simultaneously.

## 5. Required Markers (When Implemented)

```
quil.diskfs.slot.begin
quil.diskfs.slot.select.ok path_id=2
quil.diskfs.slot.write.ok bytes=N
quil.diskfs.slot.read.match ok=1
quil.diskfs.slot.done
```

Negative:
```
quil.diskfs.slot.stop_first reason=select_not_implemented
```

## 6. Safety Boundaries (Must Hold)

| Boundary | Status |
|----------|--------|
| Quil uses SLOT_STORAGE only | VERIFIED — already has it |
| Quil does not receive SLOT_BLOCK | VERIFIED — not in capability set |
| Quil does not receive MemLend | VERIFIED — no sys_grant_mem_lend |
| Quil never calls SexDrive | VERIFIED — no BLOCK_* opcodes |
| No raw LBA exposure to Quil | VERIFIED — path_id only, never LBA |
| No broad Quil redesign | VERIFIED — proof function only |

## 7. Files to Change (When Implementing)

| File | Change |
|------|--------|
| `servers/quil/src/main.rs` | Add DISKFS opcode constants, quil_diskfs_proof(), gate flag |
| (if Option A) `servers/sexfiles/src/vfs.rs` | SELECT 0x3D handler (multi-object manifest impl) |
| (if Option A) `servers/sexfiles/src/messages.rs` | OP_DISKFS_SELECT constant |

## 8. Exact Next Prompt

```
If SELECT 0x3D is implemented first (Option A):

QUIL_DISKFS_SLOT_OBJECT_PROOF_V1

Implement the Quil DiskFS proof targeting path_id=2:

1. Add DISKFS bridge opcode constants (0x38-0x3D) to servers/quil/src/main.rs.
2. Add quil_diskfs_proof() function gated by SEXOS_QUIL_DISKFS_PROOF=1.
3. SELECT path_id=2, write QUIL_BUFFER content, read back, verify match.
4. Emit markers: quil.diskfs.slot.begin → .select.ok → .write.ok → .read.match → .done.
5. Build and run:
   SEXOS_GATE_NVME=1 SEXOS_QUIL_DISKFS_PROOF=1
   ./scripts/master_runtime_gate.sh --probe 45 --keep-log
6. Verify no collision with Linen path_id=0.
7. Write docs/handoff/QUIL_DISKFS_SLOT_OBJECT_PROOF_V1.md (update this doc).

If SELECT is not yet ready (current state):

SEXFILES_DISK_MULTI_OBJECT_MANIFEST_IMPL_V1
(Implement SELECT 0x3D + V2 manifest first, then return to Quil proof.)
```
