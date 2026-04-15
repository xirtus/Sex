use crate::ipc::messages::MessageType;
use crate::ipc::{safe_pdx_call, DOMAIN_REGISTRY};
use crate::core_local::CoreLocal;

/// kernel/src/syscalls/pipe.rs
/// Phase 11: POSIX pipe routing via PDX to standalone sexc.

pub fn sys_pipe(pipe_fds: *mut u32) -> i64 {
    let sexc_pd = DOMAIN_REGISTRY.get(3).unwrap(); // Fixed PD 3 for sexc
    
    let msg = MessageType::PipeCall {
        command: 1, // PIPE_CREATE
        pipe_cap: 0,
        buffer_cap: 0,
        size: 0,
    };

    match safe_pdx_call(sexc_pd.as_ref(), 0, &msg as *const _ as u64) {
        Ok(res_ptr) => {
            let reply = unsafe { *(res_ptr as *const MessageType) };
            if let MessageType::PipeReply { pipe_cap, .. } = reply {
                unsafe {
                    *pipe_fds = pipe_cap;
                    *pipe_fds.add(1) = pipe_cap + 1;
                }
                0
            } else {
                -1
            }
        },
        Err(_) => -1,
    }
}
