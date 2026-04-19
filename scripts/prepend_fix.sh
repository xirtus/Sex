#!/bin/bash
set -euo pipefail

docker run --rm -v "$PWD":/src -w /src sexos-builder:v28 python3 -c "
import os

header = '''#![no_std]
#![feature(alloc_error_handler)]
extern crate alloc;
use core::alloc::{GlobalAlloc, Layout};

struct SimpleAlloc;
#[global_allocator]
static ALLOCATOR: SimpleAlloc = SimpleAlloc;
unsafe impl GlobalAlloc for SimpleAlloc {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 { core::ptr::null_mut() }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}
#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! { loop {} }

'''

targets = ['crates/sex-orbclient/src/lib.rs', 'servers/tuxedo/src/lib.rs', 'servers/sexdisplay/src/main.rs']

for t in targets:
    if not os.path.exists(t): continue
    with open(t, 'r') as f: content = f.read()
    if '#![no_std]' in content[:100]: continue # Skip if already fixed
    with open(t, 'w') as f: f.write(header + content)
"
