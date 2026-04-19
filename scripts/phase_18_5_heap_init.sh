#!/bin/bash
# SexOS SASOS v1.0.0 - Phase 18.5: Kernel Heap Initialization
set -euo pipefail

echo "--> 1. Implementing LockedHeap in kernel/src/memory/allocator.rs..."
cat << 'EOF' > kernel/src/memory/allocator.rs
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};
use x86_64::VirtAddr;
use linked_list_allocator::LockedHeap;
use limine::request::MemmapResponse;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100 KiB for early boot metadata

pub fn init_heap(mmap: &'static MemmapResponse, hhdm_offset: u64) {
    // Find a usable memory hole for the heap
    let usable_entry = mmap.entries().iter()
        .find(|e| e.entry_type == limine::response::EntryType::USABLE && e.length >= HEAP_SIZE as u64)
        .expect("No usable memory for kernel heap");

    let phys_addr = usable_entry.base;
    let virt_addr = phys_addr + hhdm_offset;

    unsafe {
        ALLOCATOR.lock().init(virt_addr as *mut u8, HEAP_SIZE);
    }
    
    sex_kernel::serial_println!("[SexOS] LockedHeap initialized at virtual {:#x}", virt_addr);
}

#[alloc_error_handler]
fn alloc_error_handler(layout: core::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}
EOF

echo "--> 2. Patching kernel/src/main.rs for HHDM + Heap Alignment..."
cat << 'EOF' > kernel/src/main.rs
#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;
use limine::request::{FramebufferRequest, HhdmRequest, MemmapRequest};

#[link_section = ".limine_reqs"]
static FB_REQUEST: FramebufferRequest = FramebufferRequest::new();
#[link_section = ".limine_reqs"]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();
#[link_section = ".limine_reqs"]
static MEMMAP_REQUEST: MemmapRequest = MemmapRequest::new();

mod memory;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let hhdm = HHDM_REQUEST.get_response().get().expect("hhdm failed");
    let mmap = MEMMAP_REQUEST.get_response().get().expect("mmap failed");
    let fb_res = FB_REQUEST.get_response().get().expect("fb failed");
    let fb = fb_res.framebuffers().iter().next().expect("no fb");

    // Initialize Heap
    memory::allocator::init_heap(mmap, hhdm.offset);

    // Hardware Bridge: Draw Magenta Victory Pattern
    let fb_ptr = (fb.address().as_ptr().unwrap() as u64) as *mut u32;
    unsafe {
        for y in 0..fb.height {
            for x in 0..fb.width {
                let color = (x as u32 % 255) | ((y as u32 % 255) << 8) | (0xFF << 16);
                *fb_ptr.add((y * (fb.pitch / 4) + x) as usize) = color;
            }
        }
    }

    sex_kernel::serial_println!("[SexOS] Phase 18.5: Heap Live, Pixels Armed.");
    loop {}
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    sex_kernel::serial_println!("KERNEL PANIC: {}", info);
    loop {}
}
EOF

echo "--> 3. Executing Clean Build (Docker sexos-builder:v28)..."
docker run --rm -v "$(pwd)":/src -w /src sexos-builder:v28 bash -c "
    rustup default nightly &&
    rustup component add rust-src &&
    cargo build --target x86_64-sex.json -Z build-std=core,alloc -Z json-target-spec --release
"

echo "--> 4. Packaging ISO and Launching QEMU..."
./scripts/launch_sasos_v1.sh
