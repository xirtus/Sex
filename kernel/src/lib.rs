#![no_std]
#![feature(abi_x86_interrupt)]

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
pub mod graphics;
pub mod devmgr;
pub mod hw;
pub mod keyboard;

pub const MAP_MEMORY_SYSCALL_NUM: u64 = 30;
pub const ALLOCATE_MEMORY_SYSCALL_NUM: u64 = 31;

use linked_list_allocator::LockedHeap;

pub use crate::memory::allocator;

#[global_allocator]
pub static ALLOCATOR: LockedHeap = LockedHeap::empty();

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 256 * 1024 * 1024; // 256 MiB

use limine::{BaseRevision, RequestsStartMarker, RequestsEndMarker, request::{FramebufferRequest, HhdmRequest, MemmapRequest, ModulesRequest, RsdpRequest, StackSizeRequest}};

// Limine 7.x protocol: start marker, base revision, all requests, end marker — in order
#[used]
#[link_section = ".limine_requests_start"]
static _LIMINE_START: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[link_section = ".limine_requests"]
static _LIMINE_BASE: BaseRevision = BaseRevision::new();

#[used]
#[link_section = ".limine_requests"]
pub static FB_REQUEST: FramebufferRequest = FramebufferRequest::new();
#[used]
#[link_section = ".limine_requests"]
pub static MEMMAP_REQUEST: MemmapRequest = MemmapRequest::new();
#[used]
#[link_section = ".limine_requests"]
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();
#[used]
#[link_section = ".limine_requests"]
pub static RSDP_REQUEST: RsdpRequest = RsdpRequest::new();
#[used]
#[link_section = ".limine_requests"]
pub static MODULE_REQUEST: ModulesRequest = ModulesRequest::new();

#[used]
#[link_section = ".limine_requests"]
static _STACK_SIZE: StackSizeRequest = StackSizeRequest::new(1024 * 1024); // 1 MiB

#[used]
#[link_section = ".limine_requests_end"]
static _LIMINE_END: RequestsEndMarker = RequestsEndMarker::new();

pub fn kernel_init() {
    // 1. HAL first: GDT + IDT + CR4.PKE — must precede any exception-prone code
    hal::init();

    // 2. Memory: heap + GLOBAL_VAS — must precede any Box/alloc
    let mmap = MEMMAP_REQUEST.response().unwrap_or_else(|| {
        serial_println!("FATAL: Limine memmap missing");
        loop { core::hint::spin_loop(); }
    });
    let hhdm = HHDM_REQUEST.response().unwrap_or_else(|| {
        serial_println!("FATAL: Limine HHDM missing");
        loop { core::hint::spin_loop(); }
    });
    memory::manager::init(mmap, hhdm.offset);

    // 2.5 Initialize CoreLocal for BSP (needs heap)
    unsafe { crate::core_local::CoreLocal::init(0); }
    serial_println!("kernel: CoreLocal initialized for BSP");

    // 3. Advanced Hardware (APIC + Timer)
    let rsdp_res = RSDP_REQUEST.response();
    serial_println!("kernel: RSDP Response present: {}", rsdp_res.is_some());
    let rsdp_virt = rsdp_res.map(|r| r.address as usize).unwrap_or(0);
    serial_println!("kernel: RSDP Virtual Address: {:#x}", rsdp_virt);
    
    // RSDP from Limine is virtual; convert to physical for ACPI handler
    let rsdp_phys = if rsdp_virt >= hhdm.offset as usize {
        rsdp_virt - hhdm.offset as usize
    } else {
        rsdp_virt // Assume already physical if below HHDM
    };
    serial_println!("kernel: RSDP Physical Address: {:#x}", rsdp_phys);

    hal::init_advanced(rsdp_phys as u64, hhdm.offset);

    // 4. PD bootstrap (PKU enabled, heap ready)
    init::init();

    // 5. Start Scheduler (Phase 21: Preemptive Multi-tasking)
    serial_println!("kernel: Enabling interrupts and entering scheduler loop...");
    x86_64::instructions::interrupts::enable();

    // Infinite spin loop. The timer interrupt will trigger sched.tick()
    // which will perform the first switch_to() to a userland task.
    loop {
        core::hint::spin_loop();
    }
}

#[no_mangle]
pub unsafe extern "C" fn strlen(s: *const u8) -> usize {
    let mut len = 0;
    while *s.add(len) != 0 {
        len += 1;
    }
    len
}

#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if dest < src as *mut u8 {
        memcpy(dest, src, n)
    } else if dest > src as *mut u8 {
        let mut i = n;
        while i > 0 {
            i -= 1;
            *dest.add(i) = *src.add(i);
        }
        dest
    } else {
        dest
    }
}

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = *src.add(i);
        i += 1;
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *s.add(i) = c as u8;
        i += 1;
    }
    s
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b {
            return a as i32 - b as i32;
        }
        i += 1;
    }
    0
}

pub fn bootstrap_advanced_services() {
    serial_println!("SexOS: Advanced interaction suite initialized via standalone PDs.");
}

#[cfg(test)]
use core::panic::PanicInfo;

#[cfg(test)]
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

#[cfg(test)]
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
