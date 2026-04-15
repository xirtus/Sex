#![no_std]
#![feature(abi_x86_interrupt, naked_functions)]

extern crate alloc;

pub mod serial;
pub mod vga;
pub mod memory;
pub mod pci;
pub mod pku;
pub mod slab;
pub mod cheri;
pub mod capability;
pub mod ipc;
pub mod ipc_ring;
pub mod gdt;
pub mod interrupts;
pub mod amdahl;
pub mod sunni;
pub mod throughput_test;
pub mod latency_guard;
pub mod core_local;
pub mod apic;
pub mod smp;
pub mod scheduler;
pub mod elf;
pub mod initrd;
pub mod servers;

use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 1024 * 1024; // 1 MiB

pub mod allocator {
    use super::*;
    use x86_64::{
        structures::paging::{
            mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
        },
        VirtAddr,
    };

    pub fn init_heap(
        mapper: &mut impl Mapper<Size4KiB>,
        frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    ) -> Result<(), MapToError<Size4KiB>> {
        let page_range = {
            let heap_start = VirtAddr::new(HEAP_START as u64);
            let heap_end = heap_start + HEAP_SIZE - 1u64;
            let heap_start_page = Page::containing_address(heap_start);
            let heap_end_page = Page::containing_address(heap_end);
            Page::range_inclusive(heap_start_page, heap_end_page)
        };

        for page in page_range {
            let frame = frame_allocator
                .allocate_frame()
                .ok_or(MapToError::FrameAllocationFailed)?;
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
            unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
        }

        unsafe {
            ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
        }

        Ok(())
    }
}

/// Basic Hardware Abstraction Layer (HAL) for x86_64
pub mod hal {
    use crate::serial_println;
    use crate::gdt;
    use crate::interrupts;

    pub fn init() {
        serial_println!("HAL: Initializing GDT...");
        gdt::init();
        serial_println!("HAL: Initializing IDT...");
        interrupts::init_idt();
    }
}

pub fn bootstrap_advanced_services() {
    serial_println!("SexOS: Bootstrapping Advanced Interaction Suite...");

    // 1. Security Federation (Telemetry Source)
    let srv_sec = Arc::new(crate::servers::srv_sec::SecurityFederation::new(1));
    
    // 2. Sex-Gemini (Autonomous Supervisor)
    let ai = Arc::new(crate::servers::sexgemini::SexGemini::new());
    
    // In a real system, the repair loop would run in its own PD.
    // For this bootstrap, we simulate the monitoring intent.
    serial_println!("SexOS: AI Supervisor active (Autonomous Remediation).");

    // 3. Graphics Stack
    let mut wayland = crate::servers::srv_wayland::WaylandCompositor::new();
    let _ = wayland.init();

    // 4. Font Server
    let font_srv = crate::servers::srv_font::FontServer::new();

    // 5. Hardware Input (PS/2 Keyboard)
    let mut input = crate::servers::sexinput::sexinput::new("PS/2 Keyboard");
    let _ = input.init();

    serial_println!("SexOS: Advanced Suite Ready (AI-Supervised SAS).");

    // 5. Start Autonomous Supervisor (Enters Loop)
    // Note: In a production SASOS, sexgemini runs in its own PD.
    // For this prototype, we'll initialize the repair loop.
    // crate::servers::sexgemini::SexGemini::new().run_repair_loop();
}
