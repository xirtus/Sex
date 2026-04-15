#![no_std]
#![no_main]

mod trampoline;
use trampoline::{SIGNAL_STATE, SigAction, sexc_trampoline_entry};
use libsys::pdx::{pdx_listen, pdx_reply};
use libsys::messages::MessageType;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Standalone sexc: Main syscall bridge loop
    loop {
        // Blocks with FLSCHED::park() until PDX call arrives
        let req = pdx_listen(0);
        let msg_ptr = req.arg0 as *const MessageType;
        let msg = unsafe { *msg_ptr };

        match msg {
            MessageType::IpcCall { func_id, arg0 } => {
                let res = handle_posix_syscall(func_id, arg0);
                pdx_reply(req.caller_pd, res);
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
            // Forward to sexvfs PDX (PD ID 100)
            let msg = MessageType::VfsCall { command: 1, path_ptr: 0, offset: 0, size: 4096, buffer: arg0 };
            pdx_call(100, 0, &msg as *const _ as u64, 0)
        },
        1 => { // sys_write
            let msg = MessageType::VfsCall { command: 2, path_ptr: 0, offset: 0, size: 4096, buffer: arg0 };
            pdx_call(100, 0, &msg as *const _ as u64, 0)
        },
        13 => { // sys_sigaction
            let action = unsafe { *(arg0 as *const SigAction) };
            let signum = 2; // SIGINT (Mocked mapping for prototype)
            SIGNAL_STATE.set_action(signum, action);
            0
        },
        37 => { // sys_kill
            // Routes to kernel route_signal (Syscall 16)
            unsafe { core::arch::asm!("syscall", in("rax") 16, in("rdi") arg0); }
            0
        },
        _ => u64::MAX,
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("syscall", in("rax") 24); } }
}
