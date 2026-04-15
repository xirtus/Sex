#![no_std]
#![no_main]

use libsys::pdx::{pdx_listen, pdx_reply, pdx_call};
use libsys::messages::MessageType;

/// sexdisplay: Standalone Graphical Server (Framebuffer/GPU)
/// Phase 10: Real PDX display stack.

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        // Wait-free park until graphical command or input event arrives
        unsafe { core::arch::asm!("syscall", in("rax") 24 /* SYS_PARK */); }

        let req = pdx_listen(0);
        let msg = unsafe { *(req.arg0 as *const MessageType) };

        match msg {
            MessageType::GpuCall { command, buffer_cap, width, height } => {
                let status = handle_gpu_request(command, buffer_cap, width, height);
                let reply = MessageType::GpuReply { status };
                pdx_reply(req.caller_pd, &reply as *const _ as u64);
            },
            MessageType::HIDEvent { ev_type, code, value } => {
                handle_input_event(ev_type, code, value);
                pdx_reply(req.caller_pd, 0);
            },
            _ => {
                pdx_reply(req.caller_pd, u64::MAX);
            }
        }
    }
}

fn handle_gpu_request(cmd: u32, buffer_cap: u32, width: u32, height: u32) -> i64 {
    // 1. Resolve framebuffer/command buffer physical address via PDX to kernel
    // 2. Drive hardware (Mesa/wlroots user-space dispatch)
    0
}

fn handle_input_event(ev_type: u16, code: u16, value: i32) {
    // Dispatch to Wayland/X11 clients
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("syscall", in("rax") 24); } }
}
