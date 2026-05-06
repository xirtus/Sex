use crate::backends::FsBackend;
use crate::messages;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::RwLock;

/// RamFs: Bounded RAM-backed flat filesystem.
///
/// CONTRACT (RAMFS_CONTRACT_LOCK_V1 + NAMESPACE_CAPS_V1):
/// - Max 64 files
/// - File names ≤ 24 bytes
/// - File content ≤ 4096 bytes
/// - Flat namespace (no directories)
/// - All handles validated before use
/// - Deterministic errors for all failure modes
/// - Close releases handle but file data persists (reopen by name)
/// - No POSIX semantics claim
/// - Each file has an owner PD; read/write/close/stat require matching caller_pd
/// - Server-internal (caller_pd == 0) bypasses owner checks
pub struct RamFs {
    files: RwLock<Vec<FileEntry>>,
    next_handle: AtomicU64,
}

/// A slot in the RamFS. `active: false` means handle is released
/// but name+data persists for reopen-by-name.
struct FileEntry {
    handle: u64,
    active: bool,
    owner_pd: u32,
    name: [u8; messages::RAMFS_MAX_NAME],
    name_len: u8,
    data: Vec<u8>,
}

impl RamFs {
    pub const fn new() -> Self {
        Self {
            files: RwLock::new(Vec::new()),
            next_handle: AtomicU64::new(1),
        }
    }

    /// Find an active entry by handle.
    fn find_active_by_handle<'s>(files: &'s [FileEntry], handle: u64) -> Option<&'s FileEntry> {
        files.iter().find(|e| e.active && e.handle == handle)
    }

    /// Find an active entry by handle (mutable).
    fn find_active_by_handle_mut<'s>(files: &'s mut [FileEntry], handle: u64) -> Option<&'s mut FileEntry> {
        files.iter_mut().find(|e| e.active && e.handle == handle)
    }

    /// Find any entry (active or inactive) by name.
    #[allow(dead_code)]
    fn find_any_by_name<'s>(files: &'s [FileEntry], name: &[u8]) -> Option<&'s FileEntry> {
        files.iter().find(|e| {
            e.name_len as usize == name.len() && &e.name[..name.len()] == name
        })
    }

    /// Find any entry by name (mutable).
    fn find_any_by_name_mut<'s>(files: &'s mut [FileEntry], name: &[u8]) -> Option<&'s mut FileEntry> {
        files.iter_mut().find(|e| {
            e.name_len as usize == name.len() && &e.name[..name.len()] == name
        })
    }

    /// Allocate a new file entry with the given name (initially active).
    fn allocate(&self, files: &mut Vec<FileEntry>, name: &[u8], owner_pd: u32) -> Result<u64, i64> {
        let count = files.iter().filter(|e| e.active).count();
        if count >= messages::RAMFS_MAX_FILES {
            return Err(messages::ERR_FULL);
        }

        let handle = self.next_handle.fetch_add(1, Ordering::SeqCst);
        let mut name_buf = [0u8; messages::RAMFS_MAX_NAME];
        name_buf[..name.len()].copy_from_slice(name);

        files.push(FileEntry {
            handle,
            active: true,
            owner_pd,
            name: name_buf,
            name_len: name.len() as u8,
            data: Vec::new(),
        });
        Ok(handle)
    }

    /// Check whether `caller_pd` is allowed to access an entry.
    /// caller_pd == 0 (server-internal) bypasses check.
    fn check_owner(caller_pd: u32, entry: &FileEntry) -> Result<(), i64> {
        if caller_pd == 0 || caller_pd == entry.owner_pd {
            Ok(())
        } else {
            Err(messages::ERR_PERM_DENIED)
        }
    }
}

impl FsBackend for RamFs {
    fn open(&self, name: &[u8], flags: u32, _mode: u32, caller_pd: u32) -> Result<u64, i64> {
        if name.len() > messages::RAMFS_MAX_NAME {
            return Err(messages::ERR_NAME_TOO_LONG);
        }
        if name.is_empty() {
            return Err(messages::ERR_NOT_FOUND);
        }

        let mut files = self.files.write();

        // Check if file exists by name (any state)
        if let Some(existing) = Self::find_any_by_name_mut(&mut files, name) {
            if flags & messages::RAMFS_O_EXCL != 0 {
                return Err(messages::ERR_NOT_FOUND);
            }
            // NAMESPACE_CAPS_V1: reopening requires matching owner
            Self::check_owner(caller_pd, existing)?;
            // Reactivate if inactive
            if !existing.active {
                existing.active = true;
                existing.handle = self.next_handle.fetch_add(1, Ordering::SeqCst);
            }
            return Ok(existing.handle);
        }

        // File doesn't exist: create if O_CREATE
        if flags & messages::RAMFS_O_CREATE != 0 {
            self.allocate(&mut files, name, caller_pd)
        } else {
            Err(messages::ERR_NOT_FOUND)
        }
    }

    fn read(&self, handle: u64, offset: u64, buf: &mut [u8], caller_pd: u32) -> Result<u64, i64> {
        let files = self.files.read();
        let entry = Self::find_active_by_handle(&files, handle)
            .ok_or(messages::ERR_INVALID_HANDLE)?;
        Self::check_owner(caller_pd, entry)?;

        if offset as usize >= entry.data.len() {
            return Ok(0);
        }

        let start = offset as usize;
        let available = entry.data.len() - start;
        let to_read = buf.len().min(available).min(messages::RAMFS_MAX_FILE_SIZE);
        buf[..to_read].copy_from_slice(&entry.data[start..start + to_read]);
        Ok(to_read as u64)
    }

    fn write(&self, handle: u64, offset: u64, data: &[u8], caller_pd: u32) -> Result<u64, i64> {
        let mut files = self.files.write();
        let entry = Self::find_active_by_handle_mut(&mut files, handle)
            .ok_or(messages::ERR_INVALID_HANDLE)?;
        Self::check_owner(caller_pd, entry)?;

        let start = offset as usize;
        let end = start + data.len();

        if end > messages::RAMFS_MAX_FILE_SIZE {
            return Err(messages::ERR_OVERFLOW);
        }

        if end > entry.data.len() {
            entry.data.resize(end, 0);
        }

        entry.data[start..end].copy_from_slice(data);
        Ok(data.len() as u64)
    }

    fn close(&self, handle: u64, caller_pd: u32) -> Result<(), i64> {
        let mut files = self.files.write();
        let entry = Self::find_active_by_handle_mut(&mut files, handle)
            .ok_or(messages::ERR_INVALID_HANDLE)?;
        Self::check_owner(caller_pd, entry)?;
        entry.active = false; // Release handle, keep name+data
        Ok(())
    }

    fn stat(&self, handle: u64, caller_pd: u32) -> Result<(u64, u32), i64> {
        let files = self.files.read();
        let entry = Self::find_active_by_handle(&files, handle)
            .ok_or(messages::ERR_INVALID_HANDLE)?;
        Self::check_owner(caller_pd, entry)?;
        Ok((entry.data.len() as u64, entry.name_len as u32))
    }

    fn list_at(&self, index: usize, caller_pd: u32) -> Option<(u64, u32)> {
        let files = self.files.read();
        let mut count = 0;
        for entry in files.iter() {
            if entry.active {
                if caller_pd == 0 || caller_pd == entry.owner_pd {
                    if count == index {
                        return Some((entry.handle, entry.name_len as u32));
                    }
                    count += 1;
                }
            }
        }
        None
    }

    fn len(&self, caller_pd: u32) -> usize {
        let files = self.files.read();
        if caller_pd == 0 {
            files.iter().filter(|e| e.active).count()
        } else {
            files.iter().filter(|e| e.active && e.owner_pd == caller_pd).count()
        }
    }
}

