#![no_std]
#![no_main]

mod trampoline;
mod pipe;
use trampoline::{SIGNAL_STATE, SigAction, sexc_trampoline_entry};
use pipe::handle_pipe_call;
use libsys::pdx::{pdx_listen, pdx_reply, pdx_call};
use libsys::messages::MessageType;
use libsys::sched::park_on_ring;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Spawn Trampoline thread in background (Normally would be done by kernel/init)
    
    // 2. Standalone sexc: Main syscall bridge loop
    loop {
        // Standard FLSCHED wait-free park
        park_on_ring();

        let req = pdx_listen(0);
        let msg_ptr = req.arg0 as *const MessageType;
        let msg = unsafe { *msg_ptr };

        match msg {
            MessageType::IpcCall { func_id, arg0 } => {
                let res = handle_posix_syscall(func_id, arg0);
                pdx_reply(req.caller_pd, res);
            },
            MessageType::PipeCall { command, pipe_cap, buffer_cap, size } => {
                let (status, res_size, new_cap) = handle_pipe_call(command, pipe_cap, buffer_cap, size);
                let reply = MessageType::PipeReply { status, size: res_size, pipe_cap: new_cap };
                pdx_reply(req.caller_pd, &reply as *const _ as u64);
            },
            MessageType::ProcCall { command, path_ptr, arg_ptr } => {
                let (status, pd_id) = handle_proc_call(command, path_ptr, arg_ptr);
                let reply = MessageType::ProcReply { status, pd_id };
                pdx_reply(req.caller_pd, &reply as *const _ as u64);
            },
            _ => {
                pdx_reply(req.caller_pd, u64::MAX);
            }
        }
    }
}

fn handle_posix_syscall(func_id: u32, arg0: u64) -> u64 {
    match func_id {
        0 => { // sys_read
            let msg = MessageType::VfsCall { command: 1, offset: 0, size: 4096, buffer_cap: arg0 as u32 };
            // Resolve sexvfs via capability slot 1
            pdx_call(1, 0, &msg as *const _ as u64, 0)
        },
        1 => { // sys_write
            let msg = MessageType::VfsCall { command: 2, offset: 0, size: 4096, buffer_cap: arg0 as u32 };
            pdx_call(1, 0, &msg as *const _ as u64, 0)
        },
        13 => { // sys_sigaction
            let action = unsafe { *(arg0 as *const SigAction) };
            SIGNAL_STATE.set_action(2, action); // SIGINT
            0
        },
        37 => { // sys_kill
            unsafe { core::arch::asm!("syscall", in("rax") 16, in("rdi") arg0); }
            0
        },
        _ => u64::MAX,
    }
}

fn handle_proc_call(cmd: u32, path_ptr: u64, _arg_ptr: u64) -> (i64, u32) {
    match cmd {
        1 => { // FORK
            // IPCtax: fork is a PD clone via kernel (Syscall 18)
            let res = pdx_call(1, 18, 0, 0);
            (0, res as u32)
        },
        2 => { // EXEC
            // Resolve sexnode via capability slot 2
            let res = pdx_call(2, 1, path_ptr, 0); // TRANSLATE_ELF
            (0, res as u32)
        },
        _ => (-1, 0),
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("syscall", in("rax") 24); } }
}
