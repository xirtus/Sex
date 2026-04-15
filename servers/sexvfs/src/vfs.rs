use libsys::pdx::pdx_call;

/// sexvfs: Real block I/O dispatch via pure PDX.
/// ZERO access to kernel globals (GLOBAL_ALLOCATOR / DOMAIN_REGISTRY).

pub fn ramfs_alloc_block() -> Result<u32, &'static str> {
    // Request lent-frame from sext pager (Fixed PD 2)
    // Func 0: ALLOC_FRAME
    let cap_id = pdx_call(2, 0, 0, 0);
    if cap_id == 0 { Err("ramfs: OOM from sext") } else { Ok(cap_id as u32) }
}

pub fn handle_vfs_request(cmd: u32, offset: u64, size: u64, buffer_cap: u32) -> (i64, u64) {
    match cmd {
        1 | 2 => { // FS_READ | FS_WRITE
            // Route block I/O to sexdrives (Fixed PD 200)
            // IPCtax: Pass the lent capability ID directly for zero-copy DMA
            let res = pdx_call(200, cmd, offset, buffer_cap as u64);
            (res as i64, size)
        },
        _ => (-1, 0),
    }
}
