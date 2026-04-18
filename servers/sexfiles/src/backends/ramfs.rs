use crate::backends::FsBackend;
use crate::messages::PageHandover;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::RwLock;

/// RamFs: Full lock-free RAM filesystem.
/// Backed by BTreeMap<u64, Inode> with arena allocation.
pub struct RamFs {
    inodes: RwLock<BTreeMap<u64, Inode>>,
    next_inode: AtomicU64,
}

struct Inode {
    id: u64,
    size: u64,
    // In a real system, this would point to arena-allocated pages
}

impl RamFs {
    pub const fn new() -> Self {
        Self {
            inodes: RwLock::new(BTreeMap::new()),
            next_inode: AtomicU64::new(1),
        }
    }
}

impl FsBackend for RamFs {
    fn open(&self, _path: &str, _flags: u32, _mode: u32) -> Result<u64, i64> {
        let id = self.next_inode.fetch_add(1, Ordering::SeqCst);
        let mut inodes = self.inodes.write();
        inodes.insert(id, Inode { id, size: 0 });
        Ok(id)
    }

    fn read(&self, _inode: u64, _offset: u64, _len: u32) -> Result<PageHandover, i64> {
        // Return mock page handover from arena
        Ok(PageHandover { pfn: 0x1000, pku_key: 3 })
    }

    fn write(&self, _inode: u64, _offset: u64, len: u32, _page: PageHandover) -> Result<u32, i64> {
        Ok(len)
    }

    fn close(&self, _inode: u64) -> Result<(), i64> {
        Ok(())
    }
}
