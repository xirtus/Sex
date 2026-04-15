use libsys::pdx::pdx_call;
use libsys::messages::MessageType;

/// sexvfs: Real block I/O dispatch via pure PDX.
/// Phase 9: End-to-end hardware path via sexdrives.

pub fn handle_vfs_request(cmd: u32, offset: u64, size: u64, buffer_cap: u32) -> (i64, u64) {
    match cmd {
        1 | 2 => { // FS_READ | FS_WRITE
            // 1. Resolve sexdrives PD from VFS capability (Simplified PD 200)
            let sexdrives_pd = 200;

            // 2. Map VfsCall to DmaCall for hardware driver
            let msg = MessageType::DmaCall {
                command: cmd,
                offset,
                size,
                buffer_cap,
                device_cap: 0, // Resolve from local VFS node state
            };

            // 3. Perform zero-copy PDX call to driver
            let res_ptr = pdx_call(sexdrives_pd, 0, &msg as *const _ as u64, 0);
            
            // 4. Parse DmaReply
            let reply = unsafe { *(res_ptr as *const MessageType) };
            if let MessageType::DmaReply { status, .. } = reply {
                (status, size)
            } else {
                (-1, 0)
            }
        },
        _ => (-1, 0),
    }
}
