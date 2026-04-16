use crate::backends::FsBackend;
use crate::messages::PageHandover;

/// DiskFs: Stub Disk filesystem.
pub struct DiskFs;

impl FsBackend for DiskFs {
    fn open(&self, _path: &str, _flags: u32, _mode: u32) -> Result<u64, i64> {
        Err(-1)
    }

    fn read(&self, _inode: u64, _offset: u64, _len: u32) -> Result<PageHandover, i64> {
        Err(-1)
    }

    fn write(&self, _inode: u64, _offset: u64, _len: u32, _page: PageHandover) -> Result<u32, i64> {
        Err(-1)
    }

    fn close(&self, _inode: u64) -> Result<(), i64> {
        Err(-1)
    }
}
