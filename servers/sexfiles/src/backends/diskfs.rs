use crate::backends::FsBackend;
use crate::messages;

/// DiskFs: Stub disk backend (not yet implemented).
/// Returns ERR_NOT_FOUND for all operations.
#[allow(dead_code)]
pub struct DiskFs;

impl DiskFs {
    #[allow(dead_code)]
    pub const fn new() -> Self {
        Self
    }
}

impl FsBackend for DiskFs {
    fn open(&self, _name: &[u8], _flags: u32, _mode: u32) -> Result<u64, i64> {
        Err(messages::ERR_NOT_FOUND)
    }

    fn read(&self, _handle: u64, _offset: u64, _buf: &mut [u8]) -> Result<u64, i64> {
        Err(messages::ERR_NOT_FOUND)
    }

    fn write(&self, _handle: u64, _offset: u64, _data: &[u8]) -> Result<u64, i64> {
        Err(messages::ERR_NOT_FOUND)
    }

    fn close(&self, _handle: u64) -> Result<(), i64> {
        Err(messages::ERR_NOT_FOUND)
    }

    fn stat(&self, _handle: u64) -> Result<(u64, u32), i64> {
        Err(messages::ERR_NOT_FOUND)
    }

    fn list_at(&self, _index: usize) -> Option<(u64, u32)> {
        None
    }

    fn len(&self) -> usize {
        0
    }
}
