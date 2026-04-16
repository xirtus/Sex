use crate::messages::{VfsProtocol, PageHandover};
use sex_pdx::ring::PdxReply;
use core::sync::atomic::{AtomicU64, Ordering};
use crate::backends::FsBackend;
use crate::backends::ramfs::RamFs;
use crate::backends::diskfs::DiskFs;

/// Phase 19: Handover Trampoline Architecture.

pub static IPC_OPS_TOTAL: AtomicU64 = AtomicU64::new(0);
pub static ZERO_COPY_HANDOVERS: AtomicU64 = AtomicU64::new(0);
pub static CACHE_HITS: AtomicU64 = AtomicU64::new(0);
pub static PKU_FLIPS: AtomicU64 = AtomicU64::new(0);

pub static RAMFS: RamFs = RamFs::new();
pub static DISKFS: DiskFs = DiskFs::new();

pub struct MountEntry {
    pub prefix: &'static str,
    pub backend: &'static dyn FsBackend,
}

pub struct MountTable {
    pub entries: [Option<MountEntry>; 4],
}

pub static MOUNT_TABLE: MountTable = MountTable {
    entries: [
        Some(MountEntry { prefix: "/dev", backend: &DISKFS }),
        Some(MountEntry { prefix: "/", backend: &RAMFS }),
        None, None,
    ],
};

impl MountTable {
    pub fn route(&self, path: &str) -> Option<(&'static dyn FsBackend, &str)> {
        for entry in self.entries.iter().flatten() {
            if path.starts_with(entry.prefix) {
                return Some((entry.backend, &path[entry.prefix.len()..]));
            }
        }
        None
    }
}

#[inline(always)]
pub unsafe fn pku_grant_temporary(key: u8) -> u32 {
    PKU_FLIPS.fetch_add(1, Ordering::Relaxed);
    let old_pkru: u32;
    let shift = key * 2;
    core::arch::asm!(
        "rdpkru",
        "mov {tmp}, eax",
        "and eax, {mask}",
        "xor edx, edx",
        "xor ecx, ecx",
        "wrpkru",
        mask = in(reg) !(0b11 << shift),
        tmp = out(reg) old_pkru,
        out("eax") _,
        in("ecx") 0,
    );
    old_pkru
}

#[inline(always)]
pub unsafe fn pku_restore(old_pkru: u32) {
    PKU_FLIPS.fetch_add(1, Ordering::Relaxed);
    core::arch::asm!(
        "xor edx, edx",
        "xor ecx, ecx",
        "wrpkru",
        in("eax") old_pkru,
        in("ecx") 0,
        in("edx") 0,
    );
}

#[inline(always)]
pub fn handle_vfs_message(msg: &VfsProtocol, reply: &mut PdxReply) {
    IPC_OPS_TOTAL.fetch_add(1, Ordering::Relaxed);
    match msg {
        VfsProtocol::Open { path, flags, mode } => {
            let path_str = core::str::from_utf8(path).unwrap_or("").trim_matches('\0');
            if let Some((backend, subpath)) = MOUNT_TABLE.route(path_str) {
                match backend.open(subpath, *flags, *mode) {
                    Ok(inode) => {
                        reply.status = 0;
                        reply.size = inode;
                    },
                    Err(e) => reply.status = e,
                }
            } else {
                reply.status = -2; // ENOENT
            }
        },
        VfsProtocol::HandoverRead { page, offset, len } => {
            ZERO_COPY_HANDOVERS.fetch_add(1, Ordering::Relaxed);
            // In a real system, we would use the inode/fd to call the backend
            // Here we simulate the 3-cycle PKU dance
            unsafe {
                let old_pkru = pku_grant_temporary(page.pku_key);
                // backend.read(...) logic here
                pku_restore(old_pkru);
            }
            reply.status = 0;
            reply.size = *len as u64;
        },
        VfsProtocol::HandoverWrite { page, offset, len } => {
            ZERO_COPY_HANDOVERS.fetch_add(1, Ordering::Relaxed);
            unsafe {
                let old_pkru = pku_grant_temporary(page.pku_key);
                // backend.write(...) logic here
                pku_restore(old_pkru);
            }
            reply.status = 0;
            reply.size = *len as u64;
        },
        VfsProtocol::Stats => {
            reply.status = IPC_OPS_TOTAL.load(Ordering::Relaxed) as i64;
            reply.size = ZERO_COPY_HANDOVERS.load(Ordering::Relaxed);
        }
        _ => {
            reply.status = -1;
            reply.size = 0;
        }
    }
}
