use crate::serial_println;
use crate::servers::storage;
use alloc::string::String;
use alloc::vec::Vec;

/// FAT32 BIOS Parameter Block (BPB)
#[repr(C, packed)]
pub struct Bpb {
    pub jmp: [u8; 3],
    pub oem: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub fat_count: u8,
    pub root_entries: u16,
    pub total_sectors_16: u16,
    pub media_type: u8,
    pub sectors_per_fat_16: u16,
    pub sectors_per_track: u16,
    pub head_count: u16,
    pub hidden_sectors: u32,
    pub total_sectors_32: u32,
    // FAT32 Extended fields
    pub sectors_per_fat_32: u32,
    pub flags: u16,
    pub version: u16,
    pub root_cluster: u32,
    pub fs_info_sector: u16,
    pub backup_boot_sector: u16,
    pub reserved: [u8; 12],
    pub drive_number: u8,
    pub reserved1: u8,
    pub boot_signature: u8,
    pub volume_id: u32,
    pub volume_label: [u8; 11],
    pub system_id: [u8; 8],
}

/// FAT32 Directory Entry
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct DirEntry {
    pub name: [u8; 11],
    pub attr: u8,
    pub nt_res: u8,
    pub crt_time_tenth: u8,
    pub crt_time: u16,
    pub crt_date: u16,
    pub lst_acc_date: u16,
    pub first_cluster_hi: u16,
    pub wrt_time: u16,
    pub wrt_date: u16,
    pub first_cluster_lo: u16,
    pub file_size: u32,
}

pub struct Fat32Fs {
    pub bpb: Bpb,
}

impl Fat32Fs {
    pub fn new(sector_data: &[u8]) -> Self {
        let bpb = unsafe { core::ptr::read(sector_data.as_ptr() as *const Bpb) };
        Self { bpb }
    }

    /// Translates a cluster number to a physical LBA.
    pub fn cluster_to_lba(&self, cluster: u32) -> u32 {
        let first_data_sector = self.bpb.reserved_sectors as u32 + (self.bpb.fat_count as u32 * self.bpb.sectors_per_fat_32);
        ((cluster - 2) * self.bpb.sectors_per_cluster as u32) + first_data_sector
    }

    /// Reads a directory and looks for a specific filename.
    pub fn find_entry(&self, name: &str, dir_cluster: u32) -> Option<DirEntry> {
        let lba = self.cluster_to_lba(dir_cluster);
        let mut buffer = [0u16; 256]; // 512 bytes
        
        // Read the first sector of the cluster
        storage::handle_read(1, lba as u64, 512, buffer.as_ptr() as u64);
        
        let entries = unsafe { 
            core::slice::from_raw_parts(buffer.as_ptr() as *const DirEntry, 16) 
        };

        for entry in entries {
            if entry.name[0] == 0x00 { break; } // End of directory
            if entry.name[0] == 0xE5 { continue; } // Deleted entry
            
            // Compare 8.3 filename (Simplified for prototype)
            let mut entry_name = String::new();
            for &c in &entry.name[0..8] {
                if c == b' ' || c == 0 { break; }
                entry_name.push(c as char);
            }
            
            if entry_name.to_lowercase() == name.to_lowercase() {
                return Some(*entry);
            }
        }
        None
    }
}
