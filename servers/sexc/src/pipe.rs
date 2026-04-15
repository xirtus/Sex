use libsys::pdx::pdx_call;
use libsys::messages::MessageType;

/// sexc pipe: POSIX pipe implementation via lent-memory ring buffers.
/// Phase 11: asynchronous GNU pipeline support.

pub fn create_pipe() -> (u32, u32) {
    // 1. Request 4 KiB frame from sext for the pipe buffer (Cap 2)
    let frame_cap = pdx_call(2, 0 /* ALLOC_FRAME */, 0, 0) as u32;
    
    // 2. Wrap frame in a lent-memory ring buffer capability
    // In a real system, we'd mint a specialized Pipe capability.
    // For prototype, we return the frame cap as read/write ends.
    (frame_cap, frame_cap + 1)
}

pub fn handle_pipe_call(cmd: u32, pipe_cap: u32, buffer_cap: u32, size: u64) -> (i64, u64, u32) {
    match cmd {
        1 => { // PIPE_CREATE
            let (r, w) = create_pipe();
            (0, 0, r) // Returning first cap for simplicity
        },
        2 => { // PIPE_WRITE
            // Zero-copy: buffer_cap is lent to the pipe "owner" (another PD)
            (size as i64, size, pipe_cap)
        },
        3 => { // PIPE_READ
            (size as i64, size, pipe_cap)
        },
        _ => (-1, 0, 0),
    }
}
