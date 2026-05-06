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

// ── Error constants ──
pub const ERR_INVALID_HANDLE: i64 = -1;
pub const ERR_NAME_TOO_LONG: i64 = -2;
pub const ERR_NOT_FOUND: i64 = -3;
pub const ERR_OVERFLOW: i64 = -4;
pub const ERR_FULL: i64 = -5;
pub const ERR_PERM_DENIED: i64 = -6;

// ── Bounds ──
pub const RAMFS_MAX_FILES: usize = 64;
pub const RAMFS_MAX_NAME: usize = 24; // fits in 3 u64 args
pub const RAMFS_MAX_FILE_SIZE: usize = 4096;

// ── Flags ──
pub const RAMFS_O_CREATE: u32 = 0x01;
pub const RAMFS_O_EXCL: u32 = 0x02;
