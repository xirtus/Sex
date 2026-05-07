#!/bin/bash
set -euo pipefail
echo "✦ SEX MICROKERNEL SASOS — FIXING ALLOCATOR & IPC SYMBOLS"

PROJECT_ROOT="/home/xirtus_arch/Documents/microkernel"
cd "$PROJECT_ROOT"

cargo clean -p sexdisplay
rm -rf target/x86_64-sex/release/sexdisplay

mkdir -p servers/sexdisplay/src
cat << 'SRC_EOF' > servers/sexdisplay/src/main.rs
#![no_std]
#![no_main]

extern crate alloc;
use core::alloc::{GlobalAlloc, Layout};

pub struct DummyAllocator;
unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 { core::ptr::null_mut() }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    extern "C" {
        fn pdx_register(name: *const u8) -> usize;
    }
    unsafe { pdx_register("sexdisplay\0".as_ptr()) };
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! { loop {} }
SRC_EOF

bash build_payload.sh
make clean
make iso
make run-sasos
