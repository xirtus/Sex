#![no_std]
#![feature(abi_x86_interrupt, allocator_api)]

extern crate alloc;

pub mod serial;
pub mod vga;
pub mod memory;
pub mod hal;
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
pub mod devmgr;

use linked_list_allocator::LockedHeap;

pub use crate::memory::allocator;

#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 1024 * 1024; // 1 MiB

pub fn bootstrap_advanced_services() {
    serial_println!("SexOS: Advanced interaction suite initialized via standalone PDs.");
}

#[cfg(test)]
use core::panic::PanicInfo;

pub fn test_runner(tests: &[&dyn Fn()]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test();
    }
    exit_qemu(QemuExitCode::Success);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("TEST FAILED: {}", info);
    exit_qemu(QemuExitCode::Failed);
    loop {}
}
