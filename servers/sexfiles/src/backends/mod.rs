/// FsBackend: Filesystem backend trait.
/// All operations validate handles and bounds before acting.
/// `caller_pd` is the protection domain ID of the requesting process.
/// `caller_pd == 0` means server-internal (proof/init) and bypasses owner checks.
pub trait FsBackend: Send + Sync {
    /// Open or create a file by name.
    /// On create, `caller_pd` is stored as the file's owner.
    /// On reopen, `caller_pd` must match the existing owner, or ERR_PERM_DENIED.
    fn open(&self, name: &[u8], flags: u32, mode: u32, caller_pd: u32) -> Result<u64, i64>;

    /// Read from a file handle into a buffer.
    /// Returns bytes read. Checks caller_pd matches handle owner.
    fn read(&self, handle: u64, offset: u64, buf: &mut [u8], caller_pd: u32) -> Result<u64, i64>;

    /// Write data to a file handle at offset.
    /// Returns bytes written. Checks caller_pd matches handle owner.
    fn write(&self, handle: u64, offset: u64, data: &[u8], caller_pd: u32) -> Result<u64, i64>;

    /// Close a file handle.
    /// Checks caller_pd matches handle owner.
    fn close(&self, handle: u64, caller_pd: u32) -> Result<(), i64>;

    /// Get metadata for a file handle.
    /// Checks caller_pd matches handle owner.
    fn stat(&self, handle: u64, caller_pd: u32) -> Result<(u64, u32), i64>;

    /// List files: returns (handle, name_len) at index, or None.
    /// Name content is accessible via stat().
    /// Ownership check: only returns entries owned by caller_pd (or all if caller_pd == 0).
    fn list_at(&self, index: usize, caller_pd: u32) -> Option<(u64, u32)>;

    /// Total number of open files owned by caller_pd (or all if caller_pd == 0).
    fn len(&self, caller_pd: u32) -> usize;
}

pub mod ramfs;
pub mod diskfs;
pub mod tmpfs;
