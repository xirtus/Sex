/// FsBackend: Filesystem backend trait.
/// All operations validate handles and bounds before acting.
pub trait FsBackend: Send + Sync {
    /// Open or create a file by name.
    fn open(&self, name: &[u8], flags: u32, mode: u32) -> Result<u64, i64>;

    /// Read from a file handle into a buffer.
    /// Returns bytes read.
    fn read(&self, handle: u64, offset: u64, buf: &mut [u8]) -> Result<u64, i64>;

    /// Write data to a file handle at offset.
    /// Returns bytes written.
    fn write(&self, handle: u64, offset: u64, data: &[u8]) -> Result<u64, i64>;

    /// Close a file handle.
    fn close(&self, handle: u64) -> Result<(), i64>;

    /// Get metadata for a file handle.
    fn stat(&self, handle: u64) -> Result<(u64, u32), i64>;

    /// List files: returns (handle, name_len) at index, or None.
    /// Name content is accessible via stat().
    fn list_at(&self, index: usize) -> Option<(u64, u32)>;

    /// Total number of open files.
    fn len(&self) -> usize;
}

pub mod ramfs;
pub mod diskfs;
pub mod tmpfs;
