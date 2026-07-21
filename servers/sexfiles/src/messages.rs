/// sexfiles protocol constants (PDX opcodes for slot SLOT_STORAGE=1).
/// Flat numeric namespace; no POSIX semantics.
/// Bounded: names ≤ 24 bytes, files ≤ 64, file size ≤ 4096 bytes.

/// Opcode: Create or open a file by name.
/// arg0 = name bytes 0..7  (little-endian u64)
/// arg1 = name bytes 8..15 (little-endian u64)
/// arg2 = name bytes 16..23 | (flags << 24)
/// Returns: file handle (u64) on success, error code (negative) on failure.
pub const OP_RAMFS_OPEN: u64 = 0x30;

/// Opcode: Read from a file handle.
/// arg0 = handle
/// arg1 = offset
/// arg2 = max_len (clamped to RAMFS_MAX_FILE_SIZE)
/// Returns: packed data in reply (bytes 0..7), or error code (negative).
pub const OP_RAMFS_READ: u64 = 0x31;

/// Opcode: Write to a file handle.
/// arg0 = handle
/// arg1 = offset
/// arg2 = packed_data (8 bytes of data to write)
/// Returns: bytes written (u64) on success, error code (negative) on failure.
pub const OP_RAMFS_WRITE: u64 = 0x32;

/// Opcode: Close a file handle.
/// arg0 = handle
/// Returns: 0 on success, error code (negative) on failure.
pub const OP_RAMFS_CLOSE: u64 = 0x33;

/// Opcode: List all open file handles.
/// arg0 = index (for pagination; 0 = start)
/// Returns: packed { handle: u32, size: u32 } or 0 if no more entries.
pub const OP_RAMFS_LIST: u64 = 0x34;

/// Opcode: Get file metadata by handle.
/// arg0 = handle
/// Returns: packed { size: u32, name_len: u32 } or error code (negative).
pub const OP_RAMFS_STAT: u64 = 0x35;

/// Opcode: Create a file with an explicit owner PD (proxy create for Linen bridge).
/// arg0 = name bytes 0..7  (little-endian u64)
/// arg1 = name bytes 8..15 (little-endian u64)
/// arg2 = name bytes 16..23 (lower 24 bits) | (owner_pd << 32)
/// Always creates (O_CREATE implicit). Fails with ERR_NOT_FOUND if exists.
/// Returns: file handle (u64) on success, error code (negative) on failure.
pub const OP_RAMFS_CREATE_OWNER: u64 = 0x36;

/// Opcode: Return the global RamFS object_id for an open handle.
/// arg0 = handle
/// arg1 = 0 (reserved)
/// arg2 = 0 (reserved)
/// Returns: object_id (u64, ≥1) on success, error code (negative) on failure.
/// Closes the OQ5 namespace gap: callers obtain a SexFiles-assigned ID, not
/// a client-local ID.
pub const OP_RAMFS_OBJECT_ID: u64 = 0x37;

/// Opcode: Read filename bytes for an open file handle.
/// arg0 = handle
/// arg1 = byte_offset into filename (0 = start)
/// arg2 = max_len (server clamps to 8)
/// Returns: up to 8 filename bytes packed little-endian.
///   0  = EOF (byte_offset >= name_len) — not an error.
///   negative = error (invalid handle or permission denied).
/// Caller must own the file (owner_pd match) or hold CAP_RIGHT_READ.
/// No allocation. Reads only from the fixed-size name storage.
pub const OP_RAMFS_READNAME: u64 = 0x3D;

// ── DiskFS bridge opcodes (SEXFILES_RAMFS_DISKFS_BRIDGE_ABI_PLAN_V1) ──
// Route: Linen → SLOT_STORAGE → SexFiles → DiskFS → SLOT_BLOCK → SexDrive → NVMe
// Fixed-object only: /disk/sexfiles-proof-v1

/// Opcode: Write up to 16 bytes at a byte offset into the selected DiskFS
/// object. DISKFS_V4: objects are variable-length — a write past the
/// object's current size grows it (allocating backing blocks as needed,
/// up to the per-object cap reported by OP_DISKFS_STAT / ERR_OVERFLOW
/// beyond it). Growth is NOT implicit truncation: to shrink, callers must
/// issue OP_DISKFS_TRUNCATE explicitly after the final WRITE of a save.
/// arg0 = byte_offset
/// arg1 = data bytes 0..7  (little-endian u64)
/// arg2 = data bytes 8..15 (little-endian u64)
/// Returns: bytes written (16) on success, error code (negative) on failure.
pub const OP_DISKFS_WRITE: u64 = 0x38;

/// Opcode: Read up to 8 bytes at a byte offset from the selected DiskFS
/// object. Bounded by the object's actual (variable) length — a read at
/// or beyond that length returns 0 (EOF), not stale trailing bytes.
/// arg0 = byte_offset
/// arg1 = max_len (1..8)
/// arg2 = 0 (reserved)
/// Returns: packed data (u64, bytes 0..max_len-1 LE) or error (negative).
pub const OP_DISKFS_READ: u64 = 0x39;

/// Opcode: Issue BLOCK_SYNC (NVMe FLUSH) for the DiskFS object.
/// arg0 = 0, arg1 = 0, arg2 = 0
/// Returns: 0 on success, BLOCK_ERR_NO_DEVICE (4) on QEMU, or error (negative).
pub const OP_DISKFS_FLUSH: u64 = 0x3A;

/// Opcode: Query the selected DiskFS object's metadata.
/// arg0 = 0, arg1 = 0, arg2 = 0
/// Returns: packed { flags: u32 in bits 32..63, size: u32 in bits 0..31 }
///   or error (negative). DISKFS_V4: size is the object's exact current
///   length in bytes (not a fixed constant). flags bit0=exists, bit1=writeable.
pub const OP_DISKFS_STAT: u64 = 0x3B;

/// Opcode: Return the FNV-1a 64-bit hash of the fixed DiskFS object path.
/// arg0 = 0, arg1 = 0, arg2 = 0
/// Returns: name_hash (u64) on success, or error (negative).
pub const OP_DISKFS_MANIFEST_HASH: u64 = 0x3C;

/// Opcode: Select a DiskFS object by path_id for subsequent bridge operations.
/// V1 single-client proof-only. Global state, not caller-scoped.
/// arg0 = path_id (u64): 0=/disk/sexfiles-proof-v1, 1=/disk/linen-object-v1, 2=/disk/quil-object-v1
/// arg1 = 0 (reserved), arg2 = 0 (reserved)
/// Returns: 0 on success, ERR_BAD_CMD on invalid path_id, other error on manifest failure.
pub const OP_DISKFS_SELECT: u64 = 0x3E;

/// Maximum bytes per DISKFS_WRITE call (2 u64 args = 16 bytes).
pub const DISKFS_MAX_WRITE: usize = 16;
/// Maximum bytes per DISKFS_READ call (reply u64 = 8 bytes).
pub const DISKFS_MAX_READ: usize = 8;
/// Fixed DiskFS object size in bytes.
pub const DISKFS_OBJECT_SIZE: u64 = 4096;

// ── Object status query (Phase B1) ──────────────────────────────────────
/// Opcode: Query object status by object_id.
/// arg0 = object_id (from OP_RAMFS_OBJECT_ID or known constant)
/// arg1 = 0, arg2 = 0
/// Returns: packed { exists: u8 in bit 0, size: u16 in bits 1-16,
///   generation: u32 in bits 17-48, error bits in upper 16 }
/// No reply wait required by producers — fire-and-forget status query.
/// Correlation=0 (no tx_id), durable=0 (RamFS only, not DiskFS).
pub const OP_RAMFS_STATUS: u64 = 0x3F;

/// Opcode: Run native SexObject persist proof triggered by Linen via SLOT_STORAGE.
/// arg0 = 0 (reserved), arg1 = 0, arg2 = 0
/// Returns: object_id (≥1) on success, error code (negative) on failure.
/// Route: Linen → SLOT_STORAGE → SexFiles → SexFS v0 → NVMe
pub const OP_SEXOBJECT_NATIVE_PERSIST_PROOF: u64 = 0x40;

/// Opcode: Read back existing SexObject content (no format, no write).
/// arg0 = object_id (≥1), arg1/arg2 = 0 (reserved)
/// Returns: data length (≥1) on success, error code (negative) on failure.
/// Route: Quil/Linen → SLOT_STORAGE → SexFiles → SexFS v0 → NVMe
pub const OP_SEXOBJECT_READ_BACK: u64 = 0x41;

// ── Error constants ──
pub const ERR_INVALID_HANDLE: i64 = -1;
pub const ERR_NAME_TOO_LONG: i64 = -2;
pub const ERR_NOT_FOUND: i64 = -3;
pub const ERR_OVERFLOW: i64 = -4;
pub const ERR_FULL: i64 = -5;
pub const ERR_PERM_DENIED: i64 = -6;
pub const ERR_BAD_CMD: i64 = -7;
/// DISKFS_V3: name already exists (create/rename).
pub const ERR_EXISTS: i64 = -8;
/// DISKFS_V4: manifest sector matches neither the current format, a
/// recognized legacy version, nor an all-zero (genuinely unformatted)
/// sector. Mount refuses to proceed rather than silently overwriting it.
pub const ERR_CORRUPT: i64 = -9;

// ── DISKFS_V3 dynamic object ops ────────────────────────────────────────────
/// Create object: arg1|arg2 = up to 16 name bytes LE. Reply = new path_id.
pub const OP_DISKFS_CREATE: u64 = 0x42;
/// Enumerate: arg0 = path_id, arg1 = query (0-2 name chunk, 0xFF slot info,
/// 0xFE global generation). Reply = packed (see vfs.rs).
pub const OP_DISKFS_LIST: u64 = 0x43;
/// Delete object: arg0 = path_id. System slots 0-2 protected.
pub const OP_DISKFS_DELETE: u64 = 0x47;
/// Rename object: arg0 = path_id, arg1|arg2 = new name.
pub const OP_DISKFS_RENAME: u64 = 0x48;

// ── DISKFS_V4 variable-length object ops ────────────────────────────────────
/// Truncate the selected object to an exact new length.
/// arg0 = new_length_bytes. Must be <= current size (shrink/no-op only;
/// growth happens implicitly via OP_DISKFS_WRITE past the current end).
/// Frees any blocks no longer needed and zeroes the tail of the last
/// remaining block, so a later regrow never exposes previously-shrunk
/// content. Reply = new_length_bytes on success, error (negative) on
/// failure (ERR_OVERFLOW if new_length_bytes > current size).
pub const OP_DISKFS_TRUNCATE: u64 = 0x49;

// ── Bounds ──
pub const RAMFS_MAX_FILES: usize = 64;
pub const RAMFS_MAX_NAME: usize = 24; // fits in 3 u64 args
pub const RAMFS_MAX_FILE_SIZE: usize = 4096;

// ── Flags ──
pub const RAMFS_O_CREATE: u32 = 0x01;
pub const RAMFS_O_EXCL: u32 = 0x02;
