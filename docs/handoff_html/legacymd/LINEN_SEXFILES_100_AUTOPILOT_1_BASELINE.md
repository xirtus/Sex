# LINEN_SEXFILES_100_AUTOPILOT_1_BASELINE — Current Reality Audit

**Timestamp:** 2026-05-22
**Gate:** sexfiles100.autopilot.1.baseline

---

## 1. Current Source Reality

### 1.1 Linen Files Found

| File | Size | Role |
|---|---|---|
| `servers/linen/src/main.rs` | 86726 B | Main server: opcodes, session ops, HID events, 7+ proof functions |
| `servers/linen/src/session.rs` | 7488 B | `LinenObject` struct, `Session` with 16-slot array, create/list/get/set_persisted API |
| `servers/linen/src/sexobject.rs` | 4093 B | Logical adapters: `LinenObject` → `SexObjectHeader` / `SexObjectRef` |
| `servers/linen/Cargo.toml` | — | crate name = "linen" |

### 1.2 SexFiles Dispatch Files Found

| File | Size | Role |
|---|---|---|
| `servers/sexfiles/src/main.rs` | 917 B | Entry stub |
| `servers/sexfiles/src/lib.rs` | 294 B | Module declarations |
| `servers/sexfiles/src/trampoline.rs` | 4903 B | Boot sequence with compile-time proof dispatch |
| `servers/sexfiles/src/vfs.rs` | 22030 B | VFS dispatch: RamFS (0x30-0x37) + DiskFS bridge (0x38-0x3E) + status (0x3F) |
| `servers/sexfiles/src/messages.rs` | 6235 B | All opcode constants: OP_RAMFS_* = 0x30-0x37, OP_DISKFS_* = 0x38-0x3E |
| `servers/sexfiles/src/proof.rs` | 100034 B | Built-in proofs: ramfs, diskfs, linen metadata, disk object, multi-object, etc. |
| `servers/sexfiles/src/backends/ramfs.rs` | — | RamFS backend (FsBackend trait impl) |
| `servers/sexfiles/src/backends/diskfs.rs` | — | DiskFS backend with V2 multi-object manifest |
| `servers/sexfiles/src/backends/tmpfs.rs` | — | TmpFS backend stub |
| `servers/sexfiles/src/sexobject.rs` | 3096 B | SexObject struct definitions |

### 1.3 Current VFS Protocol / Opcode Path

```
Route: Caller → SLOT_STORAGE(1) → SexFiles trampoline → vfs::handle_vfs_message()

RamFS opcodes (all active, in production path):
  0x30 OP_RAMFS_OPEN          — open/create by name, O_CREATE flag
  0x31 OP_RAMFS_READ          — read from handle at offset
  0x32 OP_RAMFS_WRITE         — write to handle at offset (8 bytes packed)
  0x33 OP_RAMFS_CLOSE         — release handle
  0x34 OP_RAMFS_LIST          — list all handles
  0x35 OP_RAMFS_STAT          — get file metadata
  0x36 OP_RAMFS_CREATE_OWNER  — create with explicit owner_pd (Linen bridge)
  0x37 OP_RAMFS_OBJECT_ID     — return global SexFiles object_id for handle

DiskFS bridge opcodes (active, behind compile-time flags):
  0x38 OP_DISKFS_WRITE        — write up to 16 bytes to DiskFS object
  0x39 OP_DISKFS_READ         — read up to 8 bytes from DiskFS object
  0x3A OP_DISKFS_FLUSH        — NVMe flush (ERR_NO_DEVICE on QEMU)
  0x3B OP_DISKFS_STAT         — query object metadata (size + flags)
  0x3C OP_DISKFS_MANIFEST_HASH — FNV-1a hash of object path
  0x3D OP_RAMFS_READNAME      — read filename bytes (Linen readback verify)
  0x3E OP_DISKFS_SELECT       — select DiskFS object by path_id (0/1/2)
  0x3F OP_RAMFS_STATUS        — query RamFS object status (Phase B1)
```

### 1.4 Current SLOT_STORAGE / SLOT_BLOCK Usage

| Constant | Value | Consumer | Purpose |
|---|---|---|---|
| `SLOT_STORAGE` | 1 | Linen, Quil, Shell | RamFS/DiskFS VFS via SexFiles |
| `SLOT_BLOCK` | 15 | SexFiles only | NVMe/block DMA via SexDrive |
| `SLOT_LINEN` | 13 | Shell | Linen app surface server calls |

**Key boundary:** Linen NEVER calls SLOT_BLOCK directly. Linen always routes through SLOT_STORAGE. The `linen_diskfs_direct_proof()` function uses DiskFS bridge opcodes (0x38-0x3E) via SLOT_STORAGE, not SLOT_BLOCK. SexFiles internally uses SLOT_BLOCK for DiskFS backend operations.

### 1.5 Current RamFS Proof Status

| Proof | Location | Status |
|---|---|---|
| `run_all_proofs()` | sexfiles/src/proof.rs:33 | COMPLETE — 7 proof checks behind SEXFILES_RAMFS_PROOF flag |
| `run_linen_sexfiles_metadata_proofs()` | sexfiles/src/proof.rs:438 | COMPLETE — Linen↔SexFiles metadata bridge |
| Linen init_session proof | linen/src/main.rs:1221 | ACTIVE — 5 boot objects persisted + readback verify |
| Linen sexfiles.metadata.proof | linen/src/main.rs:1452 | COMPLETE — behind SEXOS_LINEN_SEXFILES_METADATA_PROOF |
| Linen disk object proof | linen/src/main.rs:1589 | COMPLETE — save/load through SexFiles RamFS |
| `run_linen_disk_object_proof()` | sexfiles/src/proof.rs:527 | COMPLETE — SexFiles-side Linen payload persistence |

### 1.6 Current DiskFS Proof Status

| Proof | Location | Status |
|---|---|---|
| `run_diskfs_object_table_proofs()` | sexfiles/src/proof.rs:65 | COMPLETE — behind SEXOS_DISKFS_OBJECT_TABLE_PROOF |
| `run_diskfs_multi_object_proofs()` | sexfiles/src/proof.rs:2318 | COMPLETE — V2 multi-object including Linen/Quil objects |
| Linen diskfs.direct proof | linen/src/main.rs:1814 | COMPLETE — save/load/match through DiskFS bridge |
| Linen diskfs.slot.min proof | linen/src/main.rs:2039 | COMPLETE — V2 SELECT path_id=1 |
| SexFiles block contract proof | sexfiles/src/proof.rs:1005 | PARTIAL — contract validated, blocked on real device backend |

### 1.7 Current Daily Gate Coverage

| Gate | Status | Evidence |
|---|---|---|
| `linen_nonblocking` | SKIP/PASS | `linen.*nonblock` / `linen.open.intent` / daily summary |
| `linen_detail` | SKIP/PASS | `linen.object.*` markers count (seeds) |
| `palette_linen_available` | SKIP/PASS | Linen palette status markers |
| `linen_object_workflow` | SKIP/PASS | `[linen.object.create]` / `[linen.object.tag]` / `[linen.object.workflow.proof.done]` |
| `linen_object_persist` | SKIP/PASS | `[linen.object.persist.audit]` / `[linen.object.persist.send]` / `[linen.object.persist.proof.done]` |
| `linen_object_schema` | SKIP/PASS | `[linen.schema.kind]` / `[linen.schema.status]` |
| `spindle_linen_workflow` | SKIP/PASS | `[spindle.linen.workflow]*` markers |
| `linen_search_bridge` | SKIP/PASS | `[linen.search.bridge]*` markers |
| `linen_persist_readback` | SKIP/PASS | `[linen.persist.readback.proof.done]` ok markers |

**GAP SUMMARY:** No explicit `linen_sexfiles100_audit`, `linen_objects_list`, or `linen_ramfs_crud` gate categories exist. The existing gates cover workflow/persist/schema/search but not the structured audit trail this autopilot requires.

---

## 2. Safety Conclusion

### 2.1 What Is Safe to Implement Next (Autopilot 1)

1. **Additive proof markers** in existing `linen_init_session()` path:
   - `[linen.sexfiles100.audit.begin]` / `[linen.sexfiles100.audit.done]` wrap
   - `[linen.objects.seed]` at session init start
   - `[linen.objects.list.begin]` / `[linen.objects.list.item]` / `[linen.objects.list.done]` in object iteration loop
   - `[linen.objects.select.ok]` after first persisted object

2. **RamFS CRUD markers** in existing `linen_readback_verify()` path:
   - `[linen.ramfs.crud.begin]`
   - `[linen.ramfs.read.match]`
   - `[linen.ramfs.crud.done]`

3. **Daily gate scaffolding** for the three new categories (SKIP-safe):
   - `linen_sexfiles100_audit`
   - `linen_objects_list`
   - `linen_ramfs_crud`

These are all additive — they don't change any behavior, just emit markers. The existing paths already do the work.

### 2.2 What Is Unsafe / STOP FIRST

| Trigger | Reason |
|---|---|
| Kernel edit | Outside scope — kernel changes require full audit |
| Sex-pdx ABI edit | Would break all servers |
| New PDX message format | Requires coordination |
| SLOT_BLOCK direct calls from Linen | Violates architecture — Linen must use SLOT_STORAGE |
| SexDrive direct calls from Linen | SexDrive/storage 100 is NEXT TRACK |
| Broad VFS refactor | Risk of breaking existing RamFS/DiskFS paths |
| Dynamic directory tree | Not in scope — RamFS is flat namespace |
| Zip extraction | Not in scope |
| POSIX compatibility layer | Not in scope |
| Renderer/display/shell ownership changes | Not in scope |
| Dynamic allocation in Linen | Linen uses no-heap allocator |

---

## 3. Explicit Boundaries

1. **This is not general POSIX filesystem parity.** RamFS is a bounded flat namespace — no directories, no symlinks, no permissions beyond owner_pd.
2. **This is not dynamic directory tree completion.** The DiskFS manifest has 3 fixed entries (proof/linen/quil). No directory listing semantics.
3. **This is not zip extraction.** Not in scope this tier.
4. **This is not SexDrive/storage 100.** SexDrive/storage 100 is NEXT TRACK (separate PD, separate slots).
5. **Linen must not call SexDrive directly.** All Linen storage goes through SLOT_STORAGE → SexFiles.
6. **Linen must not use SLOT_BLOCK.** SLOT_BLOCK is SexFiles-internal only.
7. **Linen must use SexFiles/SLOT_STORAGE only.** This is the current architecture and stays that way.
8. **DiskFS bridge placement:** Already present in both Linen and SexFiles behind compile-time flags. Autopilot 1 only adds markers to the existing paths — no new DiskFS bridge code.

---

## 4. Marker Coverage Matrix (Current vs Target)

| Marker | Currently Emitted? | Source |
|---|---|---|
| `[linen.sexfiles100.audit.begin]` | NO | TARGET: wrap linen_init_session |
| `[linen.sexfiles100.audit.done]` | NO | TARGET: after session init |
| `[linen.objects.seed]` | NO | TARGET: start of linen_init_session |
| `[linen.objects.list.begin]` | YES (as `[linen.sexfiles.list.begin]`) | Already in linen_init_session |
| `[linen.objects.list.item]` | NO | TARGET: per-object in init loop |
| `[linen.objects.list.done]` | NO | TARGET: after init loop |
| `[linen.objects.select.ok]` | NO | TARGET: first persisted object |
| `[linen.ramfs.crud.begin]` | NO | TARGET: start of readback verify |
| `[linen.ramfs.read.match]` | NO | TARGET: successful readback compare |
| `[linen.ramfs.crud.done]` | NO | TARGET: after readback verify |
| `[sexfiles.bridge.diskfs.recv]` | YES | vfs.rs dispatch: 0x38-0x3C |
| `[sexfiles.bridge.diskfs.write.ok]` | YES | vfs.rs OP_DISKFS_WRITE handler |
| `[sexfiles.bridge.diskfs.read.ok]` | YES | vfs.rs OP_DISKFS_READ handler |
| `[sexfiles.bridge.diskfs.flush.err]` | YES | vfs.rs OP_DISKFS_FLUSH handler |
| `[sexfiles.bridge.diskfs.stat.ok]` | YES | vfs.rs OP_DISKFS_STAT handler |
| `[sexfiles.bridge.diskfs.manifest_hash.ok]` | YES | vfs.rs OP_DISKFS_MANIFEST_HASH handler |
| `[linen.diskfs.direct.begin]` | YES | Behind SEXOS_LINEN_DISKFS_DIRECT_PROOF |
| `[linen.diskfs.direct.write.ok]` | YES | Behind SEXOS_LINEN_DISKFS_DIRECT_PROOF |
| `[linen.diskfs.direct.read.match]` | YES | Behind SEXOS_LINEN_DISKFS_DIRECT_PROOF |
| `[linen.diskfs.direct.done]` | YES | Behind SEXOS_LINEN_DISKFS_DIRECT_PROOF |

---

## 5. Autopilot 1 Scope: Safe Additive Changes

**Files to edit:**
1. `servers/linen/src/main.rs` — Add 12 markers to existing paths
2. `scripts/daily_driver_master_gate.sh` — Add 3 gate categories

**Files NOT to edit:**
- `servers/sexfiles/*` — No changes (proofs already emit comprehensive markers)
- `crates/sex-pdx/*` — No changes
- `kernel/` — No changes

**Build/proof validation:**
- Run `./scripts/entrypoint_build.sh` if available
- Run daily driver gate against any available log

