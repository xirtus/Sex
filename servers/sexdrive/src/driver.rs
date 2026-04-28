#![feature(alloc_error_handler)]
extern crate alloc;
use alloc::vec::Vec;
#![no_std]
#![no_main]

use libsys::pdx::{pdx_listen_raw, pdx_reply};
use libsys::sched::park_on_ring;

const ABI_VERSION: u64 = 1;
const CONFIRM_SIG: u64 = 0xCAFE_BABE;

extern "C" {
    fn sys_wait_vblank_primitive();
}

fn handshake_negotiate() -> bool {
    let req = unsafe { pdx_listen_raw(0) };
    if req.arg0 != ABI_VERSION { return false; }
    unsafe { pdx_reply(req.caller_pd, CONFIRM_SIG); }
    true
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    if !handshake_negotiate() {
        loop { park_on_ring(); }
    }

    loop {
        unsafe { sys_wait_vblank_primitive(); }
        let events = event_dequeue();
        let tx = schedule_dag(events);
        execute_dag(tx);
        commit_to_ucgm(tx);
    }
}

fn event_dequeue() -> Vec<u64> { Vec::new() }
fn schedule_dag(_events: Vec<u64>) -> u64 { 0 }
fn execute_dag(_tx: u64) { }
fn commit_to_ucgm(_tx: u64) { }

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { park_on_ring(); }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { park_on_ring(); }
}
