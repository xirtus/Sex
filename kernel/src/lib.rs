#![no_std]
#![feature(abi_x86_interrupt)]

pub mod serial;
pub mod memory;
pub mod pku;

/// Basic Hardware Abstraction Layer (HAL) stubs for x86_64
pub mod hal {
    use crate::serial_println;

    pub fn init() {
        serial_println!("HAL: Initializing x86_64 stubs...");
        // TODO: Initialize GDT, IDT, Paging
    }

    pub mod interrupts {
        pub fn init_idt() {
            // TODO: Initialize IDT and forward interrupts
        }
    }

    pub mod paging {
        pub fn init_paging() {
            // TODO: Implement basic page table walker for Global VAS
        }
    }
}

/// Simple round-robin scheduler stub
pub mod scheduler {
    pub fn schedule() {
        // TODO: Implement round-robin scheduling
    }
}

/// Capability engine stubs
pub mod capability {
    pub fn verify_cap() {
        // TODO: Implement capability verification
    }
}

/// IPC Primitives (PDX) stubs
pub mod ipc {
    pub fn pdx_call() {
        // TODO: Implement PDX fast path with PKU
    }
}

pub fn init() {
    hal::init();
}
