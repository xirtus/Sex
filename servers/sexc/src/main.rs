#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // In Phase 19, this will be replaced by a PDX call to the kernel 
    // to map the shared heap region.
    loop {}
}
extern crate alloc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::boxed::Box;

mod trampoline;
mod pipe;
use pipe::handle_pipe_call;
use libsys::pdx::{pdx_listen_raw, pdx_reply, pdx_call};
use libsys::messages::{MessageType, PageHandover};
use libsys::sched::park_on_ring;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Phase 24: start the lock-free signal trampoline
    trampoline::start_signal_trampoline();

    loop {
        park_on_ring();

        let req = pdx_listen_raw(0);
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
            MessageType::ProcCall { command, path_ptr, arg_ptr, page_handover } => {
                let (status, pd_id) = handle_proc_call(command, path_ptr, arg_ptr, page_handover);
                let reply = MessageType::ProcReply { status, pd_id };
                pdx_reply(req.caller_pd, &reply as *const _ as u64);
            },
            _ => {
                pdx_reply(req.caller_pd, u64::MAX);
            }
        }
    }
}

fn handle_posix_syscall(_func_id: u32, _arg0: u64) -> u64 {
    // ... (same as before)
    u64::MAX
}

fn handle_proc_call(cmd: u32, path_ptr: u64, _arg_ptr: u64, page_handover: PageHandover) -> (i64, u32) {
    match cmd {
        1 => { // FORK
            let res = pdx_call(1, 18, 0, 0, 0);
            (0, res as u32)
        },
        2 => { // EXEC
            let res = pdx_call(2, 1, path_ptr, 0, 0); // TRANSLATE_ELF from path
            (0, res as u32)
        },
        3 => { // EXEC_PAGE
            // Resolve sexnode via capability slot 2
            // The `pdx_call` for `sexnode` needs to be updated to take a PageHandover
            // Assuming `sexnode`'s `TRANSLATE_ELF_PAGE` is command 2
            let res = pdx_call(2, 2, page_handover.pfn, page_handover.pku_key as u64, 0);
            (0, res as u32)
        },
        _ => (-1, 0),
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { park_on_ring(); }
}
