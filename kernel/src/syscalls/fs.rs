use crate::ipc::messages::MessageType;
use crate::ipc::DOMAIN_REGISTRY;
use crate::ipc::safe_pdx_call;

/// kernel/src/syscalls/fs.rs
/// Phase 3: Bridge for VFS operations via PDX.
/// Maps kernel-level FS requests to the standalone sexvfs server.

pub fn sys_read(fd: u32, buffer: u64, size: u64) -> i64 {
    route_vfs_call(1, fd as u64, size, buffer)
}

pub fn sys_write(fd: u32, buffer: u64, size: u64) -> i64 {
    route_vfs_call(2, fd as u64, size, buffer)
}

fn route_vfs_call(cmd: u32, offset: u64, size: u64, buffer: u64) -> i64 {
    // 1. Identify sexvfs PD (Hardcoded PD 100 for prototype)
    let sexvfs_pd = match DOMAIN_REGISTRY.get(100) {
        Some(pd) => pd,
        None => return -1,
    };

    // 2. Construct VfsCall message
    let msg = MessageType::VfsCall {
        command: cmd,
        path_ptr: 0, // Simplified: use fd/offset
        offset,
        size,
        buffer,
    };

    // 3. Dispatch via pure PDX (No mediation)
    match safe_pdx_call(sexvfs_pd.as_ref(), 0, &msg as *const _ as u64) {
        Ok(res_ptr) => {
            let reply = unsafe { *(res_ptr as *const MessageType) };
            if let MessageType::VfsReply { status, .. } = reply {
                status
            } else {
                -1
            }
        },
        Err(_) => -1,
    }
}
