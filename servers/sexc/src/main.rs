#![no_std]
#![no_main]

mod trampoline;
use trampoline::{SignalState, SigAction, NSIG};
use libsys::pdx::{pdx_listen, pdx_reply};
use libsys::messages::MessageType;

static SIGNAL_STATE: SignalState = SignalState::new();

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // sexc: Standalone POSIX Emulation Server
    loop {
        // Wait-free park until signal or syscall message
        let req = pdx_listen(0);
        
        let msg_ptr = req.arg0 as *const MessageType;
        let msg = unsafe { *msg_ptr };

        match msg {
            MessageType::Signal(signum) => {
                handle_signal(signum as usize);
            },
            MessageType::IpcCall { func_id, arg0 } => {
                let res = handle_syscall(func_id, arg0);
                pdx_reply(req.caller_pd, res);
            },
            _ => {
                pdx_reply(req.caller_pd, u64::MAX);
            }
        }
    }
}

fn handle_signal(signum: usize) {
    if let Some(action) = SIGNAL_STATE.get_action(signum) {
        // Dispatch on dedicated trampoline stack
        // In a real system, we switch stack pointer before calling
        crate::trampoline::sexc_trampoline_dispatch(signum as i32, action.handler);
    }
}

fn handle_syscall(func_id: u32, arg0: u64) -> u64 {
    match func_id {
        13 => { // sys_sigaction
            let action = unsafe { *(arg0 as *const SigAction) };
            let signum = 2; // Mock: SIGINT
            SIGNAL_STATE.set_action(signum, action);
            0
        },
        37 => { // sys_kill
            // In a SAS system, we route this back to kernel for delivery
            0
        },
        _ => u64::MAX,
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("syscall", in("rax") 24); } }
}
