#![no_std]
#![no_main]

use libsys::pdx::{pdx_listen, pdx_reply, pdx_call};
use libsys::messages::MessageType;

/// sexnode: Standalone Cluster Node and Dynamic Translator Manager.
/// IPCtax: Pure PDX implementation, NO globals, NO busy-wait.

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        // Blocks with FLSCHED::park() on the SPSC control ring
        unsafe { core::arch::asm!("syscall", in("rax") 24 /* SYS_PARK */); }

        let req = pdx_listen(0);
        let msg = unsafe { *(req.arg0 as *const MessageType) };

        match msg {
            MessageType::TranslatorCall { command, path_ptr, code_cap } => {
                let (status, translated_entry) = handle_translation(command, path_ptr, code_cap);
                let reply = MessageType::TranslatorReply { status, translated_entry };
                pdx_reply(req.caller_pd, &reply as *const _ as u64);
            },
            _ => {
                pdx_reply(req.caller_pd, u64::MAX);
            }
        }
    }
}

fn handle_translation(cmd: u32, _path_ptr: u64, code_cap: u32) -> (i64, u64) {
    match cmd {
        1 => { // TRANSLATE_ELF
            // 1. Discover suitable translator PD (e.g., x86_64 -> SAS native)
            // 2. Lend code_cap to the translator PD
            // 3. Request translation via PDX
            // Simulation: Return a native entry point in the SAS
            (0, 0x_4000_1000)
        },
        _ => (-1, 0),
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("syscall", in("rax") 24); } }
}
