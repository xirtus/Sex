use crate::backends::FsBackend;
use crate::messages;

/// TmpFs: Stub temp filesystem (not yet implemented).
#[allow(dead_code)]
pub struct TmpFs;

impl FsBackend for TmpFs {
    fn open(&self, _name: &[u8], _flags: u32, _mode: u32, _caller_pd: u32) -> Result<u64, i64> {
        Err(messages::ERR_NOT_FOUND)
    }

    fn read(&self, _handle: u64, _offset: u64, _buf: &mut [u8], _caller_pd: u32) -> Result<u64, i64> {
        Err(messages::ERR_NOT_FOUND)
    }

    fn write(&self, _handle: u64, _offset: u64, _data: &[u8], _caller_pd: u32) -> Result<u64, i64> {
        Err(messages::ERR_NOT_FOUND)
    }

    fn close(&self, _handle: u64, _caller_pd: u32) -> Result<(), i64> {
        Err(messages::ERR_NOT_FOUND)
    }

    fn stat(&self, _handle: u64, _caller_pd: u32) -> Result<(u64, u32), i64> {
        Err(messages::ERR_NOT_FOUND)
    }

    fn list_at(&self, _index: usize, _caller_pd: u32) -> Option<(u64, u32)> {
        None
    }

    fn len(&self, _caller_pd: u32) -> usize {
        0
    }

    fn create_with_owner(
        &self,
        _name: &[u8],
        _owner_pd: u32,
        _caller_pd: u32,
    ) -> Result<u64, i64> {
        Err(messages::ERR_NOT_FOUND)
    }
}
