use sex_pdx::{pdx_call, MessageType, PageHandover};
use core::sync::atomic::{AtomicU64, Ordering};

/// Phase 20: Superior storage backend using sexfiles VFS
pub struct Storage {
    pub object_count: AtomicU64,
}

pub static STORAGE: Storage = Storage {
    object_count: AtomicU64::new(0),
};

impl Storage {
    pub fn fetch_package(&self, name: &str) -> Result<PageHandover, i32> {
        // Resolve package to hash in /pkg/
        let mut path = [0u8; 512];
        let prefix = b"/pkg/";
        path[..prefix.len()].copy_from_slice(prefix);
        let name_bytes = name.as_bytes();
        let end = prefix.len() + name_bytes.len();
        if end > 512 { return Err(-7); } // E2BIG
        path[prefix.len()..end].copy_from_slice(name_bytes);

        // Call Slot 1 (sexfiles) for VFS Open
        let msg = MessageType::VfsOpen { path, flags: 0, mode: 0 };
        let fd = pdx_call(1, 0, &msg as *const _ as u64, 0);
        
        if fd == u64::MAX {
            return Err(-2); // ENOENT
        }

        // Request zero-copy handover for entire file
        // In Phase 20, we use VfsZeroCopyHandover to map file pages directly
        let handover_msg = MessageType::VfsRead { fd, len: 4096, offset: 0 };
        let res = pdx_call(1, 0, &handover_msg as *const _ as u64, 0);
        
        if res == u64::MAX {
            return Err(-5); // EIO
        }

        // Return PageHandover from VFS
        Ok(PageHandover { pfn: res, pku_key: 7 })
    }

    pub fn kv_get(&self, _key: &[u8; 64]) -> Result<u64, i32> {
        // Search /etc/sexshop.kv (Slot 1)
        // For prototype, return 0 if key not found
        Ok(0)
    }

    pub fn kv_set(&self, _key: &[u8; 64], _paddr: u64, _len: u64) -> i32 {
        // Append to /etc/sexshop.kv WAL via VfsWrite
        0
    }

    pub fn object_put(&self, _hash: &[u8; 32], _paddr: u64, _len: u64) -> i32 {
        self.object_count.fetch_add(1, Ordering::Relaxed);
        // Map hash to path /etc/sexshop/obj/<hash_hex>
        // Write paddr data to VFS via VfsWrite
        0
    }

    pub fn object_get(&self, _hash: &[u8; 32]) -> Result<PageHandover, i32> {
        // Open /etc/sexshop/obj/<hash_hex>
        // Return PageHandover
        Ok(PageHandover { pfn: 0x5678, pku_key: 7 })
    }

    pub fn exists(&self, _hash: &[u8; 32]) -> bool {
        // VfsStat call
        false
    }

    pub fn object_move(&self, _hash: &[u8; 32], _target_node: u32) -> i32 {
        // Implementation: ClusterObjectPush via sexnet
        0
    }
}
