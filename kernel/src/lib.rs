#![no_std]
#![feature(alloc_error_handler)]

pub mod memory;
pub mod init;

use limine::request::{MemmapRequest, HhdmRequest};

#[used]
#[link_section = ".limine_reqs"]
pub static MEMMAP_REQUEST: MemmapRequest = MemmapRequest::new();

#[used]
#[link_section = ".limine_reqs"]
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[alloc_error_handler]
fn alloc_error_handler(_layout: core::alloc::Layout) -> ! {
    loop {}
}
