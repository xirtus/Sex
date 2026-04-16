use libsys::pdx::pdx_call;
use crate::trampoline::SigAction;

/// sexc VFS Bridge: Standard POSIX VFS to sexfiles PDX.
/// Phase 19: Handover path.

pub const O_HANDOVER: u32 = 0x8000_0000;

pub fn open(path: &str, flags: u32, mode: u32) -> i32 {
    if flags & O_HANDOVER != 0 {
        // Zero-copy path: call sexfiles via PDX
        // Cap slot 1 is usually sexfiles
        pdx_call(1, 1 /* Open */, path.as_ptr() as u64, flags as u64) as i32
    } else {
        pdx_call(1, 1, path.as_ptr() as u64, flags as u64) as i32
    }
}

pub fn posix_fadvise(fd: i32, offset: u64, len: u64, advice: i32) -> i32 {
    // Phase 19: Pre-warm PKU keys hint
    pdx_call(1, 7 /* PreWarmKeys */, fd as u64, advice as u64) as i32
}
