use crate::backends::FsBackend;
use crate::messages;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::RwLock;

pub const DISKFS_BLOCK_SIZE: u32 = 4096;
pub const DISKFS_MAX_OBJECTS: usize = 16;
const DISKFS_MAGIC: u64 = 0x3156_5345_4C49_4653; // "SFILESV1"

#[derive(Clone, Copy)]
pub struct SexfilesSuperblock {
    pub magic: u64,
    pub version_major: u16,
    pub version_minor: u16,
    pub block_size: u32,
    pub fs_generation: u64,
    pub object_table_start_block: u64,
    pub object_table_entry_count: u32,
    pub feature_flags: u64,
    pub checksum: u32,
}

#[derive(Clone, Copy)]
pub struct SexfilesObjectEntry {
    pub object_id: u64,
    pub kind: u16,
    pub owner_pd: u32,
    pub rights_generation: u64,
    pub object_size_bytes: u64,
    pub first_block: u64,
    pub metadata_generation: u64,
    pub checksum: u32,
    pub in_use: bool,
}

#[derive(Clone, Copy)]
struct DiskFsState {
    mounted: bool,
    superblock: SexfilesSuperblock,
    table: [SexfilesObjectEntry; DISKFS_MAX_OBJECTS],
}

const ZERO_ENTRY: SexfilesObjectEntry = SexfilesObjectEntry {
    object_id: 0,
    kind: 0,
    owner_pd: 0,
    rights_generation: 0,
    object_size_bytes: 0,
    first_block: 0,
    metadata_generation: 0,
    checksum: 0,
    in_use: false,
};

const ZERO_SUPERBLOCK: SexfilesSuperblock = SexfilesSuperblock {
    magic: 0,
    version_major: 0,
    version_minor: 0,
    block_size: 0,
    fs_generation: 0,
    object_table_start_block: 0,
    object_table_entry_count: 0,
    feature_flags: 0,
    checksum: 0,
};

const ZERO_STATE: DiskFsState = DiskFsState {
    mounted: false,
    superblock: ZERO_SUPERBLOCK,
    table: [ZERO_ENTRY; DISKFS_MAX_OBJECTS],
};

/// DiskFs: bounded in-memory mock scaffold for V1 on-disk format lock.
/// No real block I/O path is wired yet in sexfiles->sexdrive.
#[allow(dead_code)]
pub struct DiskFs;

static DISKFS_STATE: RwLock<DiskFsState> = RwLock::new(ZERO_STATE);
static NEXT_OBJECT_ID: AtomicU64 = AtomicU64::new(1);

impl DiskFs {
    #[allow(dead_code)]
    pub const fn new() -> Self {
        Self
    }

    fn checksum_superblock(sb: &SexfilesSuperblock) -> u32 {
        (sb.magic as u32)
            ^ (sb.magic >> 32) as u32
            ^ sb.version_major as u32
            ^ sb.version_minor as u32
            ^ sb.block_size
            ^ (sb.fs_generation as u32)
            ^ (sb.fs_generation >> 32) as u32
            ^ (sb.object_table_start_block as u32)
            ^ (sb.object_table_start_block >> 32) as u32
            ^ sb.object_table_entry_count
            ^ (sb.feature_flags as u32)
            ^ (sb.feature_flags >> 32) as u32
    }

    fn checksum_entry(e: &SexfilesObjectEntry) -> u32 {
        (e.object_id as u32)
            ^ (e.object_id >> 32) as u32
            ^ e.kind as u32
            ^ e.owner_pd
            ^ (e.rights_generation as u32)
            ^ (e.rights_generation >> 32) as u32
            ^ (e.object_size_bytes as u32)
            ^ (e.object_size_bytes >> 32) as u32
            ^ (e.first_block as u32)
            ^ (e.first_block >> 32) as u32
            ^ (e.metadata_generation as u32)
            ^ (e.metadata_generation >> 32) as u32
            ^ (e.in_use as u32)
    }

    /// Format/init empty DiskFS scaffold.
    pub fn format_init_empty(&self) -> Result<(), i64> {
        let mut st = DISKFS_STATE.write();
        let mut sb = SexfilesSuperblock {
            magic: DISKFS_MAGIC,
            version_major: 1,
            version_minor: 0,
            block_size: DISKFS_BLOCK_SIZE,
            fs_generation: 1,
            object_table_start_block: 1,
            object_table_entry_count: DISKFS_MAX_OBJECTS as u32,
            feature_flags: 0,
            checksum: 0,
        };
        sb.checksum = Self::checksum_superblock(&sb);
        st.superblock = sb;
        st.table = [ZERO_ENTRY; DISKFS_MAX_OBJECTS];
        st.mounted = false;
        NEXT_OBJECT_ID.store(1, Ordering::SeqCst);
        Ok(())
    }

    /// Mount/read superblock and validate bounded format.
    pub fn mount(&self) -> Result<SexfilesSuperblock, i64> {
        let mut st = DISKFS_STATE.write();
        let sb = st.superblock;
        if sb.magic != DISKFS_MAGIC || sb.version_major != 1 || sb.block_size != DISKFS_BLOCK_SIZE {
            return Err(messages::ERR_NOT_FOUND);
        }
        if sb.object_table_entry_count as usize != DISKFS_MAX_OBJECTS {
            return Err(messages::ERR_OVERFLOW);
        }
        if Self::checksum_superblock(&sb) != sb.checksum {
            return Err(messages::ERR_OVERFLOW);
        }
        st.mounted = true;
        Ok(sb)
    }

    /// Create bounded object entry.
    pub fn create_object_entry(&self, kind: u16, owner_pd: u32) -> Result<u64, i64> {
        let mut st = DISKFS_STATE.write();
        if !st.mounted {
            return Err(messages::ERR_NOT_FOUND);
        }

        let mut free_idx: Option<usize> = None;
        let mut i = 0usize;
        while i < DISKFS_MAX_OBJECTS {
            if !st.table[i].in_use {
                free_idx = Some(i);
                break;
            }
            i += 1;
        }
        let idx = match free_idx {
            Some(v) => v,
            None => return Err(messages::ERR_FULL),
        };

        let object_id = NEXT_OBJECT_ID.fetch_add(1, Ordering::SeqCst);
        let mut e = SexfilesObjectEntry {
            object_id,
            kind,
            owner_pd,
            rights_generation: 1,
            object_size_bytes: 0,
            first_block: 0,
            metadata_generation: 1,
            checksum: 0,
            in_use: true,
        };
        e.checksum = Self::checksum_entry(&e);
        st.table[idx] = e;

        st.superblock.fs_generation = st.superblock.fs_generation.saturating_add(1);
        st.superblock.checksum = Self::checksum_superblock(&st.superblock);
        Ok(object_id)
    }

    /// Stat object entry by id.
    pub fn stat_object_entry(&self, object_id: u64) -> Result<SexfilesObjectEntry, i64> {
        let st = DISKFS_STATE.read();
        if !st.mounted || object_id == 0 {
            return Err(messages::ERR_INVALID_HANDLE);
        }
        let mut i = 0usize;
        while i < DISKFS_MAX_OBJECTS {
            let e = st.table[i];
            if e.in_use && e.object_id == object_id {
                if Self::checksum_entry(&e) != e.checksum {
                    return Err(messages::ERR_OVERFLOW);
                }
                return Ok(e);
            }
            i += 1;
        }
        Err(messages::ERR_INVALID_HANDLE)
    }
}

impl FsBackend for DiskFs {
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
}
