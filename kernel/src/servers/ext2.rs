use crate::serial_println;
use crate::servers::storage;

/// Ext2 Superblock structure.
#[repr(C, packed)]
pub struct Superblock {
    pub inodes_count: u32,
    pub blocks_count: u32,
    pub r_blocks_count: u32,
    pub free_blocks_count: u32,
    pub free_inodes_count: u32,
    pub first_data_block: u32,
    pub log_block_size: u32,
    pub log_frag_size: u32,
    pub blocks_per_group: u32,
    pub frags_per_group: u32,
    pub inodes_per_group: u32,
    pub mtime: u32,
    pub wtime: u32,
    pub mnt_count: u16,
    pub max_mnt_count: u16,
    pub magic: u16,
    pub state: u16,
    pub errors: u16,
    pub minor_rev_level: u16,
    pub lastcheck: u32,
    pub checkinterval: u32,
    pub creator_os: u32,
    pub rev_level: u32,
    pub def_resuid: u16,
    pub def_resgid: u16,
}

pub struct Ext2Fs {
    pub superblock: Superblock,
}

impl Ext2Fs {
    pub fn new(sector_data: &[u8]) -> Result<Self, &'static str> {
        let sb = unsafe { core::ptr::read(sector_data.as_ptr().add(1024) as *const Superblock) };
        if sb.magic != 0xEF53 {
            return Err("Ext2: Invalid magic number");
        }
        serial_println!("Ext2: Valid filesystem found. Blocks: {}", sb.blocks_count);
        Ok(Self { superblock: sb })
    }

    pub fn read_inode(&self, inode_id: u32) {
        serial_println!("Ext2: Reading Inode {}", inode_id);
        // 1. Calculate block group and offset
        // 2. Read Inode Table sector
        // 3. Extract metadata
    }
}
