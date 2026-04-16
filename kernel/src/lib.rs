#![no_std]
#![feature(abi_x86_interrupt, allocator_api)]

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
// pub mod throughput_test;
pub mod latency_guard;
pub mod core_local;
pub mod apic;
pub mod smp;
pub mod scheduler;
pub mod elf;
pub mod initrd;
pub mod loader;
pub mod drivers;
pub mod pd;
pub mod syscalls;
pub mod capabilities;
pub mod benchmark;
pub mod init;

use linked_list_allocator::LockedHeap;

pub use crate::memory::allocator;

#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 1024 * 1024; // 1 MiB

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
    serial_println!("SexOS: Advanced interaction suite initialized via standalone PDs.");
}
