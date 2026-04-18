use crate::messages::PageHandover;

/// FsBackend: Filesystem backend trait.
/// 100% lock-free signatures.
pub trait FsBackend: Send + Sync {
    fn open(&self, path: &str, flags: u32, mode: u32) -> Result<u64, i64>;
    fn read(&self, inode: u64, offset: u64, len: u32) -> Result<PageHandover, i64>;
    fn write(&self, inode: u64, offset: u64, len: u32, page: PageHandover) -> Result<u32, i64>;
    fn close(&self, inode: u64) -> Result<(), i64>;
}

pub mod ramfs;
pub mod diskfs;
pub mod tmpfs;
