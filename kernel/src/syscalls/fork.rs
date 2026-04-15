use crate::ipc::messages::MessageType;
use crate::ipc::{safe_pdx_call, DOMAIN_REGISTRY};

/// kernel/src/syscalls/fork.rs
/// Phase 11: fork/exec routing via PDX to standalone sexc.

pub fn sys_fork() -> i64 {
    let sexc_pd = DOMAIN_REGISTRY.get(3).unwrap();
    let msg = MessageType::ProcCall { command: 1 /* FORK */, path_ptr: 0, arg_ptr: 0 };

    match safe_pdx_call(sexc_pd.as_ref(), 0, &msg as *const _ as u64) {
        Ok(res_ptr) => {
            let reply = unsafe { *(res_ptr as *const MessageType) };
            if let MessageType::ProcReply { pd_id, .. } = reply {
                pd_id as i64
            } else {
                -1
            }
        },
        Err(_) => -1,
    }
}

pub fn sys_execve(path_ptr: u64, arg_ptr: u64) -> i64 {
    let sexc_pd = DOMAIN_REGISTRY.get(3).unwrap();
    let msg = MessageType::ProcCall { command: 2 /* EXEC */, path_ptr, arg_ptr };

    match safe_pdx_call(sexc_pd.as_ref(), 0, &msg as *const _ as u64) {
        Ok(res_ptr) => {
            let reply = unsafe { *(res_ptr as *const MessageType) };
            if let MessageType::ProcReply { status, .. } = reply {
                status
            } else {
                -1
            }
        },
        Err(_) => -1,
    }
}
