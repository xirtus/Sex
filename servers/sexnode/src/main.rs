#![no_std]
#![no_main]

use libsys::pdx::{pdx_listen, pdx_reply, pdx_call};
use libsys::messages::MessageType;
use libsys::sched::park_on_ring;

/// sexnode: Standalone Cluster Node and Dynamic Translator Manager.
/// Phase 15: Linux Driver Translation Layer + DDE-style Reuse.

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        park_on_ring();

        let req = pdx_listen(0);
        let msg = unsafe { *(req.arg0 as *const MessageType) };

        match msg {
            MessageType::TranslatorCall { command, path_ptr, code_cap } => {
                let (status, translated_entry) = handle_translation(command, path_ptr, code_cap);
                let reply = MessageType::TranslatorReply { status, translated_entry };
                pdx_reply(req.caller_pd, &reply as *const _ as u64);
            },
            MessageType::DriverLoadCall { command, driver_name_ptr } => {
                let (status, driver_pd_id) = handle_driver_load(command, driver_name_ptr);
                let reply = MessageType::DriverLoadReply { status, driver_pd_id };
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
            // 1. Resolve source code vaddr from kernel (Cap Slot 1)
            let _vaddr = pdx_call(1, 14 /* RESOLVE_VADDR */, code_cap as u64, 0);
            
            // 2. Invoke sex-gemini toolchain via sexc (Cap Slot 2) to build native code
            let build_res = pdx_call(2, 2 /* EXEC */, 0 /* "/bin/sex-cc" */, 0);
            if build_res < 0 { return (-1, 0); }
            
            // 3. Return native entry point
            (0, 0x_4000_1000)
        },
        _ => (-1, 0),
    }
}

fn handle_driver_load(cmd: u32, driver_name_ptr: u64) -> (i64, u32) {
    match cmd {
        1 => { // LOAD_LINUX_DRIVER
            // 1. Fetch Linux driver source from GitHub via sexstore (Cap slot 1)
            let fetch_res = pdx_call(1, 1 /* FETCH_PACKAGE */, driver_name_ptr, 0);
            if fetch_res < 0 { return (-1, 0); }
            
            // 2. Translate and compile via DDE wrapper (Slot 2)
            let trans_res = pdx_call(2, 2 /* EXEC */, 0 /* "dde-wrap" */, 0);
            if trans_res < 0 { return (-1, 0); }

            // 3. Load as isolated PD
            let pd_id = pdx_call(2, 17 /* SPAWN_PD */, 0, 0);
            (0, pd_id as u32)
        },
        _ => (-1, 0),
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { park_on_ring(); }
}
