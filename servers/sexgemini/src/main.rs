#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error(_layout: core::alloc::Layout) -> ! { loop {} }

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }

use sex_pdx::{safe_pdx_register, pdx_listen, MessageType, pdx_reply, PageHandover};
use sex_pdx::serial_println;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("sexgemini: started (PKEY 3 domain)");
    let _pd_slot = safe_pdx_register(b"gemini");
    loop {
        let req = pdx_listen(0);
        let msg_ptr = req.arg0 as *const sex_pdx::PdxMessage;
        if msg_ptr.is_null() { continue; }
        let msg = unsafe { &*msg_ptr };
        match msg.msg_type {
            MessageType::CompileRequest { .. } => {
                let handover = PageHandover { pfn: 0, pku_key: 2 };
                pdx_reply(req.caller_pd, handover.pfn);
            }
            MessageType::Notification { .. } => {}
            _ => {}
        }
    }
}
