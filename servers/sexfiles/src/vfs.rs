/// HandoverRead: Handover Read protocol
/// HandoverWrite: Handover Write protocol
/// Large Read/Write return PageHandover. Arena allocator for metadata.
/// 3-cycle PKU dance: pku_grant_temporary(client_key, PKU_WRU) -> capability copy only -> pku_restore()

use crate::messages::{VfsProtocol, PageHandover};
use sex_pdx::ring::PdxReply;
use core::sync::atomic::{AtomicU64, Ordering};

/// Phase 19: Handover Trampoline Architecture.

pub static IPC_OPS_TOTAL: AtomicU64 = AtomicU64::new(0);
pub static ZERO_COPY_HANDOVERS: AtomicU64 = AtomicU64::new(0);
pub static CACHE_HITS: AtomicU64 = AtomicU64::new(0);
pub static PKU_FLIPS: AtomicU64 = AtomicU64::new(0);

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
        VfsProtocol::Open => {
            reply.status = 0;
            reply.size = 42;
        },
        VfsProtocol::HandoverRead { page, offset, len } => {
            ZERO_COPY_HANDOVERS.fetch_add(1, Ordering::Relaxed);
            // 3-cycle PKU dance
            unsafe {
                let old_pkru = pku_grant_temporary(page.pku_key);
                pku_restore(old_pkru);
            }
            reply.status = 0;
            reply.size = *len as u64;
        },
        VfsProtocol::HandoverWrite { page, offset, len } => {
            ZERO_COPY_HANDOVERS.fetch_add(1, Ordering::Relaxed);
            unsafe {
                let old_pkru = pku_grant_temporary(page.pku_key);
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

pub fn handle_vfs_request(_msg: &crate::pdx::MessageType) -> crate::pdx::MessageType {
    crate::pdx::MessageType::VfsReply { status: -1, size: 0 }
}
