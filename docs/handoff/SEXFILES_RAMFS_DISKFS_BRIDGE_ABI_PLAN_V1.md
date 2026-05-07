# SEXFILES_RAMFS_DISKFS_BRIDGE_ABI_PLAN_V1

## Date

2026-05-07

## Status

PLAN COMPLETE — No implementation. STOP FIRST before code changes.

## 1. Existing RamFS Opcode Map (0x30-0x37)

All opcodes are sent through SLOT_STORAGE (slot 1). SexFiles receives them
in `vfs::handle_vfs_message()` and dispatches to the RamFS backend.

| Opcode | Name              | arg0         | arg1         | arg2                          | Reply (u64)            |
|--------|-------------------|-------------|-------------|-------------------------------|------------------------|
| 0x30   | OPEN              | name[0..7]  | name[8..15] | name[16..23] \| (flags << 24)| handle or error        |
| 0x31   | READ              | handle      | offset      | max_len (<= 4096)             | packed 8 bytes or err  |
| 0x32   | WRITE             | handle      | offset      | packed 8 bytes of data        | bytes written or err   |
| 0x33   | CLOSE             | handle      | 0           | 0                             | 0 or error             |
| 0x34   | LIST              | start_index | 0           | 0                             | packed{handle:u32,size}|
| 0x35   | STAT              | handle      | 0           | 0                             | packed{size:u32,name_len}|
| 0x36   | CREATE_OWNER      | name[0..7]  | name[8..15] | name[16..23] \| (owner<<32)  | handle or error        |
| 0x37   | OBJECT_ID         | handle      | 0           | 0                             | object_id or error     |

**Error return**: Any negative `i64` cast to `u64` in the reply value:
- `ERR_INVALID_HANDLE = -1` (0xFFFFFFFF_FFFFFFFF)
- `ERR_NAME_TOO_LONG = -2`
- `ERR_NOT_FOUND = -3` (file not found or create_owner when exists)
- `ERR_OVERFLOW = -4` (out of bounds)
- `ERR_FULL = -5` (max files reached)
- `ERR_PERM_DENIED = -6` (caller != owner and no valid cap)

**Name encoding**: Up to 24 bytes packed LE into three u64 args. arg0=bytes 0-7,
arg1=bytes 8-15, arg2=bytes 16-23 (lower 24 bits). arg2 upper bits carry flags
(OPEN: flags<<24) or owner_pd (CREATE_OWNER: owner_pd<<32).

**Data transfer**: WRITE packs 8 bytes inline in arg2. READ returns 8 bytes
packed in reply u64. Maximum file size is 4096 bytes = 512 WRITE calls for a
full file.

**Ownership**: OPEN with O_CREATE sets owner=caller_pd automatically.
CREATE_OWNER sets owner explicitly but requires caller_pd == 0 (server-internal)
or caller_pd == owner_pd. All read/write/close/stat require caller_pd match
or valid capability record.

**Current routing**: All 0x30-0x37 route to `backend: &dyn FsBackend = &RAMFS`.
There is NO DiskFS routing path currently.

## 2. Linen's Current Save/Load Route

Linen (PD 7) has SLOT_STORAGE capability. It does NOT have SLOT_BLOCK or
SLOT_BUF_LEND. Its save/load flow:

```
Save:
  OP_RAMFS_OPEN (0x30, O_CREATE) → handle
  OP_RAMFS_WRITE (0x32) × N      → 8 bytes per call, N = ceil(len/8)
  OP_RAMFS_CLOSE (0x33)           → data persists

Load:
  OP_RAMFS_OPEN (0x30, flags=0)   → reopen by name
  OP_RAMFS_READ (0x31) × N        → 8 bytes per call
  OP_RAMFS_CLOSE (0x33)           → cleanup
```

Linen uses `pdx_storage_sync(opcode, arg0, arg1, arg2) -> Result<u64, i64>`
which does `pdx_call(SLOT_STORAGE, ...)` and spins for the IPC reply (type_id=0x1).

**Critical constraint**: Linen cannot call `sys_grant_mem_lend()` because it
does not have a domain slot for SLOT_BLOCK. It cannot pass a buffer VA to
SexFiles. All data must travel inline in the PDX message registers (arg0..arg2
+ reply u64).

## 3. DiskFS Fixed Object Helpers (Current)

The DiskFS file ops are internal to SexFiles. They require a `buf_va`
parameter — a MemLend buffer mapping obtained via `sys_grant_mem_lend(SLOT_BLOCK, 4096, SLOT_BUF_LEND)`.

```
DiskFs::diskfs_lookup_path(path: &[u8], buf_va: u64)
  -> Result<DiskManifestEntryV1, u64>

DiskFs::diskfs_write_object(path: &[u8], offset: u64, data: &[u8], buf_va: u64)
  -> Result<u64, u64>   // bytes written

DiskFs::diskfs_read_object(path: &[u8], offset: u64, out: &mut [u8], buf_va: u64)
  -> Result<u64, u64>   // bytes read

DiskFs::diskfs_fsync()
  -> u64                // 0 on success, BLOCK_ERR_NO_DEVICE on QEMU
```

The fixed path is `/disk/sexfiles-proof-v1` (DISKFS_MANIFEST_OBJECT_PATH),
backed by LBAs 2038-2045 (8 sectors = 4096 bytes), with the manifest at
LBA 2046. Lookup hashes the path with FNV-1a 64-bit and matches against
the single manifest entry.

## 4. Proposed DiskFS Bridge Opcodes

### Design Principles

1. **Inline-only data transfer**: Linen has no SLOT_BLOCK, no MemLend.
   SexFiles mediates all buffer grants internally.
2. **Handle-less for V1**: V1 supports only one fixed object
   (`/disk/sexfiles-proof-v1`). No dynamic create/delete. Opcodes omit
   handle parameters — the object is implicit.
3. **Same error encoding**: Negative i64 cast to u64, matching RamFS conventions.
4. **Same reply pattern**: Single u64 reply value, matching `pdx_storage_sync()`.
5. **No new allocations in Linen**: Same `pdx_call(SLOT_STORAGE, opcode, ...)`
   pattern. No new crate dependencies.

### Opcode Table

| Opcode | Name                 | arg0         | arg1 (optional) | arg2            | Reply (u64)             |
|--------|----------------------|-------------|-----------------|-----------------|-------------------------|
| 0x38   | DISKFS_WRITE         | byte_offset | data_lo (bytes 0..7 of payload) | data_hi (bytes 8..15 of payload) | bytes_written (0..16) or error |
| 0x39   | DISKFS_READ          | byte_offset | max_len (0..16, clamped) | reserved (0) | packed data (up to 16 bytes LE) or error |
| 0x3A   | DISKFS_FLUSH         | 0           | 0               | 0               | 0 or BLOCK_ERR_*        |
| 0x3B   | DISKFS_STAT          | 0           | 0               | 0               | packed{size:u32,flags:u32} or error |
| 0x3C   | DISKFS_MANIFEST_HASH | 0           | 0               | 0               | name_hash (FNV-1a 64-bit) of the fixed object path |

### 4A. DISKFS_WRITE (0x38) — Write 16 bytes at offset

```
arg0 = byte_offset: u64          — 0..4095, must be aligned to 16
arg1 = data_lo: u64              — bytes 0..7 of payload (LE)
arg2 = data_hi: u64              — bytes 8..15 of payload (LE)

Reply:
  Ok(bytes_written)              — 16 on success (or 0..15 for partial boundary write)
  Err(ERR_OVERFLOW)              — offset >= 4096 or offset+16 > 4096
  Err(ERR_INVALID_HANDLE)        — unknown object (should not happen with fixed path)
  Err(ERR_PERM_DENIED)           — V2 when ownership is added; V1 passes all callers
```

**Server-side logic:**
1. SexFiles grants MemLend buffer once (lazily, on first DISKFS_WRITE).
2. Copies 16 bytes from arg1/arg2 into the buffer at `byte_offset`.
3. Calls `DiskFs::diskfs_write_object(path, byte_offset, &inline_data[..16], buf_va)`.
4. Returns bytes written or error.

**Buffer reuse**: The buffer is granted on first write and held for the
lifetime of the server. Read-modify-write handles partial sector writes.
The `diskfs_write_object` internally does RMW per affected sector.

**Alignment note**: V1 requires 16-byte alignment for efficient inline
packing (arg1 + arg2 = 16 bytes per call). This is optional — the server
can handle misaligned writes by adjusting the slice length. The client
SHOULD align to 16 for efficiency.

### 4B. DISKFS_READ (0x39) — Read up to 16 bytes at offset

```
arg0 = byte_offset: u64          — 0..4095
arg1 = max_len: u64              — 1..16, clamped to min(16, 4096-offset)
arg2 = reserved: u64             — must be 0

Reply:
  Ok(packed_data)                — up to 16 bytes packed LE in reply u64
  Err(ERR_OVERFLOW)              — offset >= 4096
  Err(ERR_INVALID_HANDLE)        — unknown object
  Err(ERR_PERM_DENIED)           — V2 ownership gate
```

**Server-side logic:**
1. Reads up to 16 bytes from `DiskFs::diskfs_read_object(path, byte_offset, &mut buf[..n], buf_va)`.
2. Packs bytes into reply u64 (bytes[0] at bit 0..7, etc.).
3. Returns packed value.

**Zero-length reads**: If `max_len == 0`, return `Ok(0)`.
**Past-end reads**: If `byte_offset >= 4096`, return `Err(ERR_OVERFLOW)`.
If `byte_offset + max_len > 4096`, clamp max_len.

### 4C. DISKFS_FLUSH (0x3A) — Issue BLOCK_SYNC

```
arg0 = 0, arg1 = 0, arg2 = 0

Reply:
  Ok(0)                           — flush completed
  Err(BLOCK_ERR_NO_DEVICE)        — QEMU NVMe does not emulate FLUSH (honest error)
  Err(BLOCK_ERR_TIMEOUT)          — flush timed out
```

**Server-side logic:**
Calls `DiskFs::diskfs_fsync()` which calls `diskfs_block_sync()`.
On QEMU, returns `BLOCK_ERR_NO_DEVICE`. Data is still intact — the
error is honest about lack of media durability guarantee.

### 4D. DISKFS_STAT (0x3B) — Query object metadata

```
arg0 = 0, arg1 = 0, arg2 = 0

Reply:
  Ok(packed)                      — size:u32 in bits 0..31, flags:u32 in bits 32..63
  Err(ERR_INVALID_HANDLE)         — object not found
```

**Packed format**: `(flags as u64) << 32 | (size as u64)`
- `size`: object size in bytes (4096 for the fixed proof object)
- `flags`: bit0 = exists (1), bit1 = writeable (1), others reserved (0)

### 4E. DISKFS_MANIFEST_HASH (0x3C) — Return object path hash

```
arg0 = 0, arg1 = 0, arg2 = 0

Reply:
  Ok(name_hash)                   — FNV-1a 64-bit hash of /disk/sexfiles-proof-v1
  Err(ERR_NOT_FOUND)             — manifest not readable
```

Returns the FNV-1a hash of `DISKFS_MANIFEST_OBJECT_PATH`. Allows clients
to verify they are talking to the correct object without embedding the
hash constant. Useful for multi-object V2.

## 5. Request/Reply Packing Details

### Write Packing (arg1 + arg2 → 16 bytes inline)

```
Linen builds 16 bytes of payload:
  let mut data_lo: u64 = 0;
  let mut data_hi: u64 = 0;
  for i in 0..8 {
      data_lo |= (payload[offset + i] as u64) << (i * 8);
  }
  for i in 8..16 {
      data_hi |= (payload[offset + i] as u64) << ((i - 8) * 8);
  }

Then calls:
  pdx_storage_sync(0x38, offset, data_lo, data_hi)
```

### Read Packing (reply u64 → up to 16 bytes)

```
let reply = pdx_storage_sync(0x39, offset, max_len, 0)?;
let bytes = reply.to_le_bytes();
readback[offset..offset+max_len].copy_from_slice(&bytes[..max_len]);
```

## 6. Object Identity Handling (V1)

V1 supports exactly ONE fixed object: `/disk/sexfiles-proof-v1`.

- **Path**: `b"/disk/sexfiles-proof-v1"` (24 bytes, fits in 3 u64 name args)
- **Name hash**: FNV-1a 64-bit of the path, computed by `DiskFs::proof_manifest_name_hash()`
- **Size**: 4096 bytes (8 sectors × 512 bytes)
- **LBAs**: 2038 through 2045 (8 consecutive sectors)
- **Manifest LBA**: 2046 (single entry pointing to above range)
- **Flags**: READ (0x1) | WRITE (0x2)

**No dynamic path strings**: DISKFS_WRITE/READ do not take a path argument.
The object is implicit. If V2 needs multiple objects, add `OP_DISKFS_OPEN` (0x3D)
to select by name or hash.

**No file handle**: Unlike RamFS, V1 DiskFS bridge is stateless between calls.
Each DISKFS_WRITE/READ operates directly on the fixed object. There is no
open/close lifecycle. This simplifies the protocol and avoids handle management
on the server side.

## 7. Buffer Semantics (Server-Internal)

```
┌─────────────┐     SLOT_STORAGE     ┌─────────────┐     SLOT_BLOCK      ┌──────────┐
│   Linen     │ ──→ (0x38-0x3C) ──→ │  SexFiles   │ ──→ diskfs_*() ──→ │ SexDrive │
│   (PD 7)    │ ←── reply u64  ←─── │  (PD 11)    │ ←── status     ←── │ (PD 2)   │
└─────────────┘                     └─────────────┘                     └──────────┘
                                          │
                                          │ sys_grant_mem_lend(SLOT_BLOCK, 4096, SLOT_BUF_LEND)
                                          │
                                     ┌─────────┐
                                     │ Kernel  │
                                     │ MemLend │
                                     └─────────┘
```

**Server-side buffer management:**
1. On first DISKFS_WRITE or DISKFS_READ: SexFiles calls
   `sys_grant_mem_lend(SLOT_BLOCK, 4096, SLOT_BUF_LEND)` → gets `buf_va`.
2. Stores `buf_va` in a `static` for reuse across all subsequent calls.
3. For writes: copies inline data from arg1/arg2 into the buffer, calls
   `diskfs_write_object()` with RMW.
4. For reads: calls `diskfs_read_object()` into the buffer, copies
   requested bytes to reply.
5. Buffer is never released — one grant for server lifetime.
6. Linen NEVER sees `buf_va`. It only sends/receives inline u64 values.

**Concurrency**: V1 is single-threaded (no_std, no async). No concurrent
PDX messages during proof execution. If concurrent clients arrive later,
the buffer must be protected by a lock or per-call grant.

## 8. Negative Cases

| Case                        | Opcode | Args                                | Expected Reply         |
|-----------------------------|--------|--------------------------------------|------------------------|
| Write at offset 4096       | 0x38   | offset=4096, data=...                | ERR_OVERFLOW (-4)      |
| Write at offset 4080       | 0x38   | offset=4080, data_hi=non-zero       | ERR_OVERFLOW (end >4096)|
| Write at offset 4095       | 0x38   | offset=4095, data_lo=non-zero       | ERR_OVERFLOW (only 1 byte fit, rejected)|
| Read at offset 4096        | 0x39   | offset=4096, max_len=1              | ERR_OVERFLOW (-4)      |
| Read at offset 4095        | 0x39   | offset=4095, max_len=1              | Ok(packed 1 byte)      |
| Read at offset 0, len=0    | 0x39   | offset=0, max_len=0                 | Ok(0)                  |
| Read at offset 0, len=17   | 0x39   | offset=0, max_len=17                | Ok(16 bytes, clamped)  |
| Flush on QEMU              | 0x3A   | —                                    | ERR_NO_DEVICE (4)      |
| Bad opcode (e.g., 0x3F)    | any    | any                                  | ERR_NOT_FOUND (-3)     |
| Stat unknown object        | 0x3B   | —                                    | ERR_INVALID_HANDLE (-1)|
| Flush with flags            | 0x3A   | arg0=1 (non-zero)                    | Ok(0) (flags ignored in V1) |

## 9. Proof Markers

New markers emitted by the VFS handler in `handle_vfs_message()`:

### DISKFS_WRITE handlers (0x38)
- `[sexfiles.diskfs.bridge.write] offset=N len=16` — received write request
- `[sexfiles.diskfs.bridge.write.ok] offset=N written=16` — write succeeded
- `[sexfiles.diskfs.bridge.write.err] offset=N code=E` — write failed
- `[sexfiles.diskfs.bridge.buf_grant] buf_va=0x...` — first buffer grant

### DISKFS_READ handlers (0x39)
- `[sexfiles.diskfs.bridge.read] offset=N max_len=M` — received read request
- `[sexfiles.diskfs.bridge.read.ok] offset=N read=M` — read succeeded
- `[sexfiles.diskfs.bridge.read.err] offset=N code=E` — read failed

### DISKFS_FLUSH handlers (0x3A)
- `[sexfiles.diskfs.bridge.flush]` — received flush request
- `[sexfiles.diskfs.bridge.flush.ok]` — flush completed
- `[sexfiles.diskfs.bridge.flush.err] status=E` — flush failed

Linen-side markers (in `run_linen_disk_object_proof()` updated to use new opcodes):
- `[linen.diskfs.bridge.save.request]` — save through new opcodes
- `[linen.diskfs.bridge.save.write] offset=N` — each write call
- `[linen.diskfs.bridge.save.ok]` — save complete
- `[linen.diskfs.bridge.load.request]` — load through new opcodes
- `[linen.diskfs.bridge.load.read] offset=N` — each read call
- `[linen.diskfs.bridge.load.match]` — match verified
- `[linen.diskfs.bridge.flush] status=E` — flush result
- `[linen.diskfs.bridge.done]` — proof complete

## 10. VFS Routing Changes (Implementation Notes)

Currently `handle_vfs_message()` has:

```rust
let backend: &dyn FsBackend = &RAMFS;
match type_id {
    messages::OP_RAMFS_OPEN => { ... backend.open(...) ... }
    // ... all 0x30-0x37 route to backend (RAMFS) ...
    _ => messages::ERR_NOT_FOUND as u64,
}
```

The new opcodes (0x38-0x3C) do NOT go through `FsBackend`. They need inline
handling in the match statement that directly calls `DiskFs` methods:

```rust
messages::OP_DISKFS_WRITE => {
    // arg0=offset, arg1=data_lo, arg2=data_hi
    // Internal: grant buf_va once, copy data, call diskfs_write_object
    handle_diskfs_write(arg0, arg1, arg2)
}
messages::OP_DISKFS_READ => {
    // arg0=offset, arg1=max_len
    handle_diskfs_read(arg0, arg1)
}
messages::OP_DISKFS_FLUSH => {
    DiskFs::diskfs_fsync()
}
messages::OP_DISKFS_STAT => {
    handle_diskfs_stat()
}
messages::OP_DISKFS_MANIFEST_HASH => {
    DiskFs::proof_manifest_name_hash(DISKFS_MANIFEST_OBJECT_PATH)
}
```

The buffer VA is managed by a helper module:

```rust
// New in vfs.rs or a new bridge.rs module
use core::sync::atomic::{AtomicU64, Ordering};

static DISKFS_BUF_VA: AtomicU64 = AtomicU64::new(0);

fn get_or_grant_buf_va() -> u64 {
    let mut va = DISKFS_BUF_VA.load(Ordering::Relaxed);
    if va == 0 || va == u64::MAX {
        va = sys_grant_mem_lend(SLOT_BLOCK, 4096, SLOT_BUF_LEND);
        if va != 0 && va != u64::MAX {
            DISKFS_BUF_VA.store(va, Ordering::Relaxed);
            serial_println!("[sexfiles.diskfs.bridge.buf_grant] buf_va={:#x}", va);
        }
    }
    va
}
```

## 11. Files to Change (Implementation)

| File                              | Change                                            |
|-----------------------------------|---------------------------------------------------|
| `servers/sexfiles/src/messages.rs`| Add `OP_DISKFS_WRITE` (0x38) through `OP_DISKFS_MANIFEST_HASH` (0x3C) |
| `servers/sexfiles/src/vfs.rs`     | Add inline handlers for 0x38-0x3C with buffer management |
| `servers/sexfiles/src/trampoline.rs` | Wire `SEXOS_LINEN_DISK_OBJECT_PROOF` if not done |
| `servers/linen/src/main.rs`       | Update `run_linen_disk_object_proof()` to use 0x38-0x3A |
| `docs/handoff/LINEN_DISK_OBJECT_PROOF_V1.md` | Update to reflect direct bridge |

**Files NOT changed:**
- `crates/sex-pdx/src/lib.rs` — no ABI edits (SLOT_STORAGE unchanged)
- `kernel/src/` — no kernel changes
- `apps/sexdrive/src/main.rs` — no sexdrive changes
- `servers/sexfiles/src/backends/diskfs.rs` — existing helpers unchanged
- `servers/sexfiles/src/proof.rs` — existing proofs unchanged (new proof uses bridge)

## 12. STOP FIRST Conditions

| Condition                                      | Met? | Resolution                                    |
|------------------------------------------------|------|-----------------------------------------------|
| Bridge requires kernel cap changes             | NO   | Uses existing SLOT_STORAGE, no new kernel caps |
| Bridge requires Linen to receive SLOT_BLOCK    | NO   | All block I/O stays inside SexFiles           |
| Bridge requires raw cross-PD pointers          | NO   | All data inline in u64 registers              |
| Bridge requires general filesystem allocator   | NO   | Fixed single object, no dynamic allocation     |
| Bridge requires dynamic path IPC               | NO   | Object is implicit, no path strings in messages|
| New opcode ABI change                          | YES  | STOP FIRST — review against sexos_contract.toml |

The new opcodes (0x38-0x3C) are an extension of the SLOT_STORAGE ABI
surface. While they do not change sex-pdx or kernel code, they DO change
the protocol between Linen and SexFiles. Per `sexos_contract.toml`, any
new opcode requires ABI review before implementation.

## 13. Exact Next Prompt

```
SEXFILES_RAMFS_DISKFS_BRIDGE_IMPL_V1

Implement the DiskFS bridge opcodes 0x38-0x3C as designed in
SEXFILES_RAMFS_DISKFS_BRIDGE_ABI_PLAN_V1.md.

STOP FIRST:
- Confirm sexos_contract.toml ABI review is complete.
- Confirm no kernel, sex-pdx, or sexdrive changes are needed.

Implementation steps:
1. Add opcodes 0x38-0x3C to messages.rs.
2. Add inline handlers in vfs.rs with buffer management.
3. Update Linen run_linen_disk_object_proof() to use new opcodes.
4. Add bridge proof markers.
5. Build with SEXOS_LINEN_DISK_OBJECT_PROOF=1.
6. Run master_runtime_gate.sh --probe 25 --keep-log.
7. Verify: linen.diskfs.bridge.save.ok, linen.diskfs.bridge.load.match.
8. Write docs/handoff/SEXFILES_RAMFS_DISKFS_BRIDGE_IMPL_V1.md.
```
